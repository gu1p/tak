# Minimal remote-eligible project to exercise tak -> takd -> Tor -> takd.
# `tak run hello --remote` forces remote containerized placement; with
# MOCK_CONTAINER the remote node simulates the container. Tiny on purpose: a
# ~1 KB workspace upload removes the "large upload over a slow onion" variable.
SPEC = module_spec(
    project_id="remote_probe",
    tasks=[
        task(
            "hello",
            doc="Echo a marker so we can confirm remote (mock) container execution.",
            steps=[cmd("sh", "-c", "echo TAK_REMOTE_OK_MARKER && uname -a")],
        ),
    ],
)
SPEC
