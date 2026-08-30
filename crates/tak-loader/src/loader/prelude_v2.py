_TAK_NEXT_SESSION_ID = 0


def _next_session_id():
    global _TAK_NEXT_SESSION_ID
    _TAK_NEXT_SESSION_ID = _TAK_NEXT_SESSION_ID + 1
    return "__tak_v2_session_" + str(_TAK_NEXT_SESSION_ID)


def _or_empty_list(value):
    return value if value is not None else []


def _or_empty_dict(value):
    return value if value is not None else {}


def _dep_to_label(value):
    if isinstance(value, str):
        return value
    if isinstance(value, dict):
        name = value.get("name")
        if isinstance(name, str):
            if name.startswith("//") or name.startswith(":"):
                return name
            return ":" + name
    raise TypeError("dependency must be a label string or a task object")


def _normalize_deps(value):
    if value is None:
        return []
    if isinstance(value, list):
        return [_dep_to_label(item) for item in value]
    return [_dep_to_label(value)]


def Defaults(container=None, execution=None, retry=None, queue=None, tags=None, pass_env=None):
    return {
        "__tak_kind": "defaults_v2",
        "queue": queue,
        "retry": retry,
        "container": container,
        "execution": execution,
        "tags": _or_empty_list(tags),
        "pass_env": _or_empty_list(pass_env),
    }


def module_spec(tasks, *, spec_version, limiters=None, queues=None, exclude=None, includes=None, defaults=None, project_id=None):
    if spec_version != 2:
        raise TypeError("module_spec requires literal spec_version=2")
    if defaults is not None and defaults.get("__tak_kind") != "defaults_v2":
        raise TypeError("module_spec(defaults=...) expects Defaults(...)")
    return {
        "__tak_kind": "module_spec_v2",
        "spec_version": spec_version,
        "project_id": project_id,
        "tasks": _or_empty_list(tasks),
        "limiters": _or_empty_list(limiters),
        "queues": _or_empty_list(queues),
        "exclude": _or_empty_list(exclude),
        "includes": _or_empty_list(includes),
        "defaults": defaults if defaults is not None else Defaults(),
    }


def RemoteSelection_Balanced():
    return {"kind": "balanced"}


def RemoteSelection_Sequential():
    return {"kind": "sequential"}


def RemoteSelection_RoundRobin():
    return {"kind": "round_robin"}


def Transport_DirectHttps():
    return {"kind": "direct"}


def Transport_Any():
    return {"kind": "any"}


def Transport_TorOnionService():
    return {"kind": "tor"}


def Affinity_PreferSameNode(group):
    return {"kind": "prefer_same_node", "group": group}


def Affinity_RequireSameNode(group):
    return {"kind": "require_same_node", "group": group}


def SessionReuse_Workspace():
    return {"kind": "workspace"}


def SessionReuse_Paths(paths):
    return {"kind": "paths", "paths": _or_empty_list(paths)}


def SessionReuse_SharedWorkspace(max_parallel_tasks):
    return {"kind": "shared_workspace", "max_parallel_tasks": max_parallel_tasks}


def SessionReuse_Container():
    return {"kind": "container"}


def session(name=None, execution=None, reuse=None, context=None, affinity=None):
    return {
        "__tak_kind": "session_v2",
        "id": _next_session_id(),
        "name": name,
        "execution": execution,
        "reuse": reuse if reuse is not None else SessionReuse_Workspace(),
        "context": context,
        "affinity": affinity,
    }


def Execution_Local(container=None, session=None):
    return {
        "kind": "local_only",
        "local": {"container": container, "session": session},
    }


def Execution_Remote(pool=None, required_tags=None, required_capabilities=None, transport=None, container=None, selection=None, session=None):
    return {
        "kind": "remote_only",
        "remote": {
            "pool": pool,
            "required_tags": _or_empty_list(required_tags),
            "required_capabilities": _or_empty_list(required_capabilities),
            "transport": transport,
            "container": container,
            "selection": selection if selection is not None else RemoteSelection_Balanced(),
            "session": session,
        },
    }


def path(value):
    return {"kind": "path", "value": value}


def glob(value):
    return {"kind": "glob", "value": value}


def task(name, deps=None, steps=None, needs=None, queue=None, retry=None, timeout_s=None, context=None, outputs=None, execution=None, use_session=None, cascade_session=False, tags=None, doc=None, idempotent=False, pass_env=None, affinity=None):
    if execution is not None and use_session is not None:
        raise TypeError("task `" + str(name) + "` cannot use both execution and use_session")
    return {
        "name": name,
        "deps": _normalize_deps(deps),
        "steps": _or_empty_list(steps),
        "needs": _or_empty_list(needs),
        "queue": queue,
        "retry": retry,
        "timeout_s": timeout_s,
        "context": context,
        "outputs": _or_empty_list(outputs),
        "execution": execution,
        "session": use_session,
        "cascade_session": cascade_session,
        "tags": _or_empty_list(tags),
        "doc": doc if doc is not None else "",
        "idempotent": idempotent,
        "pass_env": _or_empty_list(pass_env),
        "affinity": affinity,
    }


def cmd(*argv, cwd=None, env=None):
    return {"kind": "cmd", "argv": list(argv), "cwd": cwd, "env": _or_empty_dict(env)}


def script(path, *argv, interpreter=None, cwd=None, env=None):
    return {
        "kind": "script",
        "path": path,
        "argv": list(argv),
        "interpreter": interpreter,
        "cwd": cwd,
        "env": _or_empty_dict(env),
    }
