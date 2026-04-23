#!/usr/bin/env python3

from pathlib import Path
import errno
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import hashlib
import os
import re
import shutil
import subprocess
import sys
from urllib.parse import urlsplit
import venv


def python_in_venv(venv_dir):
    if os.name == "nt":
        return venv_dir / "Scripts" / "python.exe"
    return venv_dir / "bin" / "python"


def bin_in_venv(venv_dir, name):
    if os.name == "nt":
        return venv_dir / "Scripts" / f"{name}.exe"
    return venv_dir / "bin" / name


def slugify(value):
    return re.sub(r"[^a-z0-9]+", "-", value.lower()).strip("-")


def toml_string(value):
    escaped = value.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


CLIENT_DISCONNECT_ERRNOS = {
    errno.EPIPE,
    errno.ECONNRESET,
    errno.ECONNABORTED,
}


def is_client_disconnect_error(err):
    if isinstance(err, (BrokenPipeError, ConnectionResetError, ConnectionAbortedError)):
        return True
    return isinstance(err, OSError) and err.errno in CLIENT_DISCONNECT_ERRNOS


class QuietSimpleHTTPRequestHandler(SimpleHTTPRequestHandler):
    def copyfile(self, source, outputfile):
        try:
            super().copyfile(source, outputfile)
        except OSError as err:
            if is_client_disconnect_error(err):
                self.close_connection = True
                return
            raise

    def handle_one_request(self):
        try:
            super().handle_one_request()
        except OSError as err:
            if is_client_disconnect_error(err):
                self.close_connection = True
                return
            raise

    def finish(self):
        try:
            if not self.wfile.closed:
                try:
                    self.wfile.flush()
                except OSError as err:
                    if not is_client_disconnect_error(err):
                        raise
        finally:
            try:
                self.wfile.close()
            except OSError as err:
                if not is_client_disconnect_error(err):
                    raise
            try:
                self.rfile.close()
            except OSError as err:
                if not is_client_disconnect_error(err):
                    raise


class QuietThreadingHTTPServer(ThreadingHTTPServer):
    daemon_threads = True
    allow_reuse_address = True


WORKSPACE_CRATES = [
    (
        "tak-core",
        "tak_core",
        [
            ("tak-core::model", "model/index.html"),
            ("tak-core::endpoint", "endpoint/index.html"),
        ],
    ),
    ("tak-loader", "tak_loader", []),
    ("tak-proto", "tak_proto", []),
    ("tak-runner", "tak_runner", []),
    (
        "tak-exec",
        "tak_exec",
        [
            ("tak-exec::container_runtime", "container_runtime/index.html"),
            ("tak-exec::step_runner", "step_runner/index.html"),
        ],
    ),
    (
        "takd",
        "takd",
        [
            ("takd::daemon", "daemon/index.html"),
            ("takd::service", "service/index.html"),
            ("takd::agent", "agent/index.html"),
        ],
    ),
    (
        "tak",
        "tak",
        [
            ("tak::cli", "cli/index.html"),
            ("tak::docs", "docs/index.html"),
            ("tak::web", "web/index.html"),
        ],
    ),
]


def resolve_checkout_tak_bin():
    checkout_target_dir = Path(os.environ.get("CARGO_TARGET_DIR", "target"))
    if not checkout_target_dir.is_absolute():
        checkout_target_dir = Path.cwd() / checkout_target_dir
    default_tak_bin = checkout_target_dir / "debug" / f"tak{'.exe' if os.name == 'nt' else ''}"
    return Path(os.environ.get("TAK_BIN", str(default_tak_bin))).resolve()


def prepare_docs_tree(out_dir, tak_bin, dev_addr):
    docs_dir = out_dir / "docs"
    site_dir = out_dir / "site"
    config_file = out_dir / "zensical.toml"
    dump_file = out_dir / "docs.dump.md"

    out_dir.mkdir(parents=True, exist_ok=True)
    shutil.rmtree(docs_dir, ignore_errors=True)
    shutil.rmtree(site_dir, ignore_errors=True)
    docs_dir.mkdir(parents=True, exist_ok=True)

    dump = subprocess.check_output([str(tak_bin), "docs", "dump"], text=True)
    dump_file.write_text(dump, encoding="utf-8")
    (docs_dir / "index.md").write_text(dump, encoding="utf-8")

    lines = dump.splitlines()
    headings = [line[3:].strip() for line in lines if line.startswith("## ")]

    for index, heading in enumerate(headings):
        next_heading = headings[index + 1] if index + 1 < len(headings) else None
        section = []
        in_section = False
        for line in lines:
            if line == f"## {heading}":
                in_section = True
            if in_section:
                if next_heading and line == f"## {next_heading}":
                    break
                section.append(line)
        (docs_dir / f"{slugify(heading)}.md").write_text(
            "\n".join(section) + "\n",
            encoding="utf-8",
        )

    internals_lines = [
        "# Internals",
        "",
        "Full internal Rust API reference generated from workspace crates with private items included.",
        "",
        "## Crate Roots",
        "",
    ]
    for display_name, rustdoc_name, _ in WORKSPACE_CRATES:
        internals_lines.append(f"- [{display_name}](rustdoc/{rustdoc_name}/index.html)")

    internals_lines.extend(
        [
            "",
            "## Key Areas",
            "",
        ]
    )
    for _, rustdoc_name, pages in WORKSPACE_CRATES:
        for label, relative_path in pages:
            internals_lines.append(f"- [{label}](rustdoc/{rustdoc_name}/{relative_path})")

    internals_lines.extend(
        [
            "",
            "## DSL Surface",
            "",
            "The Python DSL constructors, functions, typed stubs, and examples remain in the generated `TASKS.py API Surface` page.",
            "",
        ]
    )
    (docs_dir / "internals.md").write_text("\n".join(internals_lines), encoding="utf-8")

    nav_entries = ['  {"Home" = "index.md"}']
    nav_entries.append('  {"Internals" = "internals.md"}')
    nav_entries.extend(
        f'  {{{toml_string(heading)} = {toml_string(f"{slugify(heading)}.md")}}}'
        for heading in headings
    )
    config_lines = [
        "[project]",
        'site_name = "Tak Docs"',
        'site_description = "Source-generated workspace reference"',
        'docs_dir = "docs"',
        'site_dir = "site"',
        "use_directory_urls = false",
        f"dev_addr = {toml_string(dev_addr)}",
        "nav = [",
        ",\n".join(nav_entries),
        "]",
        "",
        "[project.theme]",
        'variant = "modern"',
    ]
    config_file.write_text("\n".join(config_lines) + "\n", encoding="utf-8")

    return site_dir


def ensure_zensical(venv_dir, zensical_spec):
    venv_python = python_in_venv(venv_dir)
    if not venv_python.exists():
        venv.EnvBuilder(with_pip=True).create(venv_dir)

    zensical_bin = bin_in_venv(venv_dir, "zensical")
    subprocess.run(
        [str(venv_python), "-m", "ensurepip", "--upgrade"],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

    install_result = subprocess.run(
        [
            str(venv_python),
            "-m",
            "pip",
            "install",
            "--upgrade",
            "--force-reinstall",
            "--disable-pip-version-check",
            zensical_spec,
        ],
        check=False,
    )

    if install_result.returncode != 0:
        if not zensical_bin.exists():
            raise SystemExit(
                "failed to install Zensical and no existing binary is available in the docs venv"
            )
        print(
            "generate_docs_wiki: warning: could not refresh Zensical; using the existing venv install",
            file=sys.stderr,
        )

    return zensical_bin


def build_rustdoc_site(out_dir, site_dir):
    cargo_target_dir = out_dir / "cargo-doc"
    cargo_doc_root = cargo_target_dir / "doc"
    rustdoc_site_dir = site_dir / "rustdoc"

    cargo_doc_command = ["cargo", "doc", "--no-deps", "--document-private-items", "--lib"]
    for display_name, _, _ in WORKSPACE_CRATES:
        cargo_doc_command.extend(["-p", display_name])

    cargo_doc_env = os.environ.copy()
    cargo_doc_env["CARGO_TARGET_DIR"] = str(cargo_target_dir)
    subprocess.run(cargo_doc_command, check=True, env=cargo_doc_env)

    shutil.rmtree(rustdoc_site_dir, ignore_errors=True)
    shutil.copytree(cargo_doc_root, rustdoc_site_dir, dirs_exist_ok=True)

    for _, rustdoc_name, pages in WORKSPACE_CRATES:
        crate_root = rustdoc_site_dir / rustdoc_name / "index.html"
        if not crate_root.is_file():
            raise SystemExit(f"missing rustdoc crate root: {crate_root}")
        for _, relative_path in pages:
            page = rustdoc_site_dir / rustdoc_name / relative_path
            if not page.is_file():
                raise SystemExit(f"missing rustdoc page: {page}")

    return rustdoc_site_dir


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: docs_wiki.py <build|serve>")

    mode = sys.argv[1]
    out_dir = Path(os.environ.get("TAK_DOCS_WIKI_DIR", ".tmp/docs-wiki")).resolve()
    venv_dir = Path(os.environ.get("TAK_DOCS_WIKI_VENV_DIR", str(out_dir / ".venv")))
    zensical_spec = os.environ.get("ZENSICAL_SPEC", "zensical>=0.0.33,<0.1")
    dev_addr = os.environ.get("TAK_DOCS_WIKI_DEV_ADDR", "localhost:8000")
    tak_bin = resolve_checkout_tak_bin()

    site_dir = prepare_docs_tree(out_dir, tak_bin, dev_addr)
    zensical_bin = ensure_zensical(venv_dir, zensical_spec)
    config_file = out_dir / "zensical.toml"
    subprocess.run(
        [str(zensical_bin), "build", "--config-file", str(config_file)],
        check=True,
    )
    rustdoc_site_dir = build_rustdoc_site(out_dir, site_dir)

    if mode == "serve":
        parsed_addr = urlsplit(f"http://{dev_addr}")
        host = parsed_addr.hostname or "localhost"
        port = parsed_addr.port or 8000
        print(f"generate_docs_wiki: serving at http://{dev_addr}")
        sys.stdout.flush()
        handler = partial(QuietSimpleHTTPRequestHandler, directory=str(site_dir))
        with QuietThreadingHTTPServer((host, port), handler) as server:
            try:
                server.serve_forever()
            except KeyboardInterrupt:
                pass
    elif mode == "build":
        index_file = site_dir / "index.html"
        internals_file = site_dir / "internals.html"
        rustdoc_index = rustdoc_site_dir / "tak_core" / "index.html"
        if not index_file.is_file():
            raise SystemExit(f"missing generated index: {index_file}")
        if not internals_file.is_file():
            raise SystemExit(f"missing generated internals page: {internals_file}")
        if not rustdoc_index.is_file():
            raise SystemExit(f"missing generated rustdoc index: {rustdoc_index}")
        digest = hashlib.sha256(index_file.read_bytes()).hexdigest()[:12]
        print(
            f"generate_docs_wiki: wrote {index_file} sha256={digest} "
            f"with internals at {internals_file}"
        )
    else:
        raise SystemExit(f"unsupported docs wiki mode: {mode}")


if __name__ == "__main__":
    main()
