_TAK_NEXT_SESSION_ID = 0
_TAK_NEXT_POLICY_ID = 0

_Scope_Machine = "machine"
_Scope_User = "user"
_Scope_Project = "project"
_Scope_Worktree = "worktree"

_Hold_During = "during"
_Hold_AtStart = "at_start"

_QueueDiscipline_Fifo = "fifo"
_QueueDiscipline_Priority = "priority"


def _next_session_id():
    global _TAK_NEXT_SESSION_ID
    _TAK_NEXT_SESSION_ID = _TAK_NEXT_SESSION_ID + 1
    return "__tak_v2_session_" + str(_TAK_NEXT_SESSION_ID)


def _next_policy_id():
    global _TAK_NEXT_POLICY_ID
    _TAK_NEXT_POLICY_ID = _TAK_NEXT_POLICY_ID + 1
    return "__tak_v2_policy_" + str(_TAK_NEXT_POLICY_ID)


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
    """Build inherited version 2 task defaults, including environment dependencies."""
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
    """Declare a version 2 module boundary loaded from one TASKS.py file."""
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
    """Prefer least-loaded matching workers and spread ties deterministically."""
    return {"kind": "balanced"}


def RemoteSelection_Sequential():
    """Try matching workers in inventory order."""
    return {"kind": "sequential"}


def RemoteSelection_RoundRobin():
    """Rotate through matching workers with a daemon-persisted cursor."""
    return {"kind": "round_robin"}


def Transport_DirectHttps():
    """Require direct HTTPS transport for a remote worker."""
    return {"kind": "direct"}


def Transport_Any():
    """Allow daemon inventory to select direct or Tor transport."""
    return {"kind": "any"}


def Transport_TorOnionService():
    """Require Tor onion-service transport for a remote worker."""
    return {"kind": "tor"}


def Container_Resources(cpu_cores, memory_mb):
    """Declare CPU and memory reservations for containerized execution."""
    return {
        "__tak_kind": "container_resources",
        "cpu_cores": float(cpu_cores),
        "memory_mb": int(memory_mb),
    }


def _normalize_container_resources(resources):
    if resources is None:
        return None
    if resources.get("__tak_kind") != "container_resources":
        raise TypeError("resources must be created with Container.Resources(...)")
    return {
        "cpu_cores": resources.get("cpu_cores"),
        "memory_mb": resources.get("memory_mb"),
    }


def Container_Image(image, mounts=None, env=None, resources=None):
    """Run one job inside a prebuilt container image."""
    return {
        "kind": "containerized",
        "image": str(image),
        "dockerfile": None,
        "build_context": None,
        "command": None,
        "mounts": _or_empty_list(mounts),
        "env": _or_empty_dict(env),
        "resource_limits": _normalize_container_resources(resources),
    }


def Container_Dockerfile(dockerfile, build_context=None, mounts=None, env=None, resources=None):
    """Build a job container from a workspace Dockerfile."""
    return {
        "kind": "containerized",
        "image": None,
        "dockerfile": dockerfile if isinstance(dockerfile, dict) else path(dockerfile),
        "build_context": build_context if isinstance(build_context, dict) else path(build_context or "."),
        "command": None,
        "mounts": _or_empty_list(mounts),
        "env": _or_empty_dict(env),
        "resource_limits": _normalize_container_resources(resources),
    }


def Affinity_PreferSameNode(group):
    """Prefer placing affinity-group tasks on the same worker."""
    return {"kind": "prefer_same_node", "group": group}


def Affinity_RequireSameNode(group):
    """Require every affinity-group task to use the same worker."""
    return {"kind": "require_same_node", "group": group}


def SessionReuse_Workspace():
    """Create an isolated workspace for each task in the session."""
    return {"kind": "workspace"}


def SessionReuse_Paths(paths):
    """Reuse selected private-CAS cache paths between session tasks."""
    return {"kind": "paths", "paths": _or_empty_list(paths)}


def SessionReuse_SharedWorkspace(max_parallel_tasks):
    """Share one session workspace with bounded task concurrency."""
    return {"kind": "shared_workspace", "max_parallel_tasks": max_parallel_tasks}


def SessionReuse_Container():
    """Fuse a cascaded task graph into one container job."""
    return {"kind": "container"}


def session(name=None, execution=None, reuse=None, context=None, affinity=None):
    """Declare per-run session reuse, placement, context, and affinity constraints."""
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
    """Force daemon-owned scheduling onto the local worker."""
    return {
        "kind": "local_only",
        "local": {"reason": "", "container": container, "session": session},
    }


def Execution_Remote(pool=None, required_tags=None, required_capabilities=None, transport=None, container=None, selection=None, session=None):
    """Force daemon-owned scheduling onto matching remote workers."""
    return {
        "kind": "remote_only",
        "remote": {
            "reason": "",
            "pool": pool,
            "required_tags": _or_empty_list(required_tags),
            "required_capabilities": _or_empty_list(required_capabilities),
            "transport": transport,
            "container": container,
            "selection": selection if selection is not None else RemoteSelection_Balanced(),
            "session": session,
        },
    }


def PolicyContext(task_side_effecting=False, local_cpu_percent=0.0):
    """Provide the authored facts exposed to a custom placement policy."""
    return {
        "task": {"side_effecting": bool(task_side_effecting)},
        "local": {"cpu_percent": float(local_cpu_percent)},
    }


POLICY_CONTEXT = PolicyContext()


def Decision_local(reason="DEFAULT_LOCAL_POLICY", container=None):
    """Return an explicit local placement decision from a custom policy."""
    decided = Execution_Local(container=container)
    decided["local"]["reason"] = str(reason)
    return decided


def Decision_remote(reason="DEFAULT_REMOTE_POLICY", pool=None, required_tags=None, required_capabilities=None, transport=None, container=None):
    """Return an explicit remote placement decision from a custom policy."""
    decided = Execution_Remote(
        pool=pool,
        required_tags=required_tags,
        required_capabilities=required_capabilities,
        transport=transport,
        container=container,
    )
    decided["remote"]["reason"] = str(reason)
    return decided


def Execution_Decide(policy):
    """Resolve a Python placement policy before submitting candidates to takd."""
    if isinstance(policy, str):
        raise TypeError("Execution.Decide(...) expects a callable policy, not a string")
    decided = policy(POLICY_CONTEXT)
    if not isinstance(decided, dict) or decided.get("kind") not in ["local_only", "remote_only"]:
        raise TypeError("policy function must return Decision.local/remote")
    return decided


def Execution_FirstAvailable(placements, doc=None, name=None):
    """Submit concrete placement candidates in authored preference order."""
    placements = _or_empty_list(placements)
    if len(placements) == 0:
        raise TypeError("Execution.FirstAvailable requires at least one placement")
    for placement in placements:
        if not isinstance(placement, dict) or placement.get("kind") not in ["local_only", "remote_only"]:
            raise TypeError("Execution.FirstAvailable accepts local and remote placements")
    return {
        "kind": "first_available",
        "policy_id": str(name) if name is not None else _next_policy_id(),
        "placements": placements,
    }


def path(value):
    """Reference one workspace path in inputs, outputs, or session caches."""
    return {"kind": "path", "value": value}


def glob(value):
    """Reference one workspace glob in outputs or session caches."""
    return {"kind": "glob", "value": value}


def gitignore():
    """Reuse repository gitignore rules as a CurrentState ignore source."""
    return {"kind": "gitignore"}


def CurrentState(roots=None, ignored=None, include=None):
    """Capture current workspace contents as an execution input snapshot."""
    return {
        "roots": _or_empty_list(roots),
        "ignored": _or_empty_list(ignored),
        "include": _or_empty_list(include),
    }


def task(name, deps=None, steps=None, needs=None, queue=None, retry=None, timeout_s=None, context=None, outputs=None, execution=None, use_session=None, cascade_session=False, tags=None, doc=None, idempotent=False, pass_env=None, affinity=None):
    """Declare one version 2 task and its daemon execution contract."""
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


def need(name, slots=1, scope=_Scope_Project, hold=_Hold_During):
    """Request limiter capacity while one task is scheduled."""
    return {
        "limiter": {"name": name, "scope": scope},
        "slots": slots,
        "hold": hold,
    }


def queue_use(name, scope=_Scope_Machine, slots=1, priority=0):
    """Request admission through one declared queue."""
    return {
        "queue": {"name": name, "scope": scope},
        "slots": slots,
        "priority": priority,
    }


def resource(name, capacity, unit=None, scope=_Scope_Machine):
    """Declare a capacity-based coordination limiter."""
    return {
        "kind": "resource",
        "name": name,
        "scope": scope,
        "capacity": capacity,
        "unit": unit,
    }


def lock(name, scope=_Scope_Machine):
    """Declare an exclusive coordination limiter."""
    return {"kind": "lock", "name": name, "scope": scope}


def queue_def(name, slots, discipline=_QueueDiscipline_Fifo, scope=_Scope_Machine):
    """Declare daemon queue capacity; use slots to bound active work."""
    return {
        "name": name,
        "scope": scope,
        "slots": slots,
        "discipline": discipline,
    }


def rate_limit(name, burst, refill_per_second, scope=_Scope_Machine):
    """Declare a token-bucket task-start limiter."""
    return {
        "kind": "rate_limit",
        "name": name,
        "scope": scope,
        "burst": burst,
        "refill_per_second": refill_per_second,
    }


def process_cap(name, max_running, match=None, scope=_Scope_Machine):
    """Declare a limit for matching external processes."""
    return {
        "kind": "process_cap",
        "name": name,
        "scope": scope,
        "max_running": max_running,
        "match": match,
    }


def fixed(seconds):
    """Use a fixed delay between retry attempts."""
    return {"kind": "fixed", "seconds": seconds}


def exp_jitter(min_s=1, max_s=60, jitter="full"):
    """Use bounded exponential jitter between retry attempts."""
    return {
        "kind": "exp_jitter",
        "min_s": min_s,
        "max_s": max_s,
        "jitter": jitter,
    }


def retry(attempts=1, on_exit=None, backoff=None):
    """Declare retry count, exit-code matching, and backoff."""
    return {
        "attempts": attempts,
        "on_exit": _or_empty_list(on_exit),
        "backoff": backoff if backoff is not None else fixed(0),
    }


def cmd(*argv, cwd=None, env=None):
    """Run one command step with optional cwd and explicit environment values."""
    return {"kind": "cmd", "argv": list(argv), "cwd": cwd, "env": _or_empty_dict(env)}


def script(path, *argv, interpreter=None, cwd=None, env=None):
    """Run one workspace script with optional interpreter, cwd, and environment."""
    return {
        "kind": "script",
        "path": path,
        "argv": list(argv),
        "interpreter": interpreter,
        "cwd": cwd,
        "env": _or_empty_dict(env),
    }
