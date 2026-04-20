# Class Diagram

```mermaid
classDiagram
    class TaskProvider {
        <<interface>>
        +discover() TaskSet
    }

    class TaskTemplate {
        +doc
        +steps
        +needs
        +queue
        +retry
        +timeout_s
        +context
        +outputs
        +execution
        +tags
    }

    class FoundTask {
        +key
        +name
        +template
        +deps
        +task_deps
        +metadata
    }

    class TaskSet {
        +provider
        +tasks
        +doc
        +where(...)
        +without(...)
        +with_execution(...)
        +with_retry(...)
        +with_timeout(...)
        +with_needs(...)
        +with_queue(...)
        +with_tags(...)
        +materialize(...)
    }

    class GroupMode {
        <<enum>>
        PARALLEL
        SERIAL
    }

    class GroupPlan {
        +by_metadata
        +by_name_separator
        +by_name_depth
        +mode
        +aggregate_prefix
    }

    class MaterializePlan {
        +prefix
        +separator
        +root_task
        +grouping
    }

    TaskProvider --> TaskSet : returns
    TaskSet "1" *-- "*" FoundTask
    FoundTask *-- TaskTemplate
    GroupPlan --> GroupMode : uses
    MaterializePlan --> GroupPlan : optional
    TaskSet --> MaterializePlan : uses
```

## Explanation

- [[TaskProvider]] is the entry point. Its only required method is [[TaskProvider.discover]].
- [[TaskProvider.discover]] returns one [[TaskSet]].
- [[TaskSet]] contains many [[FoundTask]] values.
- Each [[FoundTask]] owns one [[TaskTemplate]].
- [[TaskSet.materialize]] consumes one [[MaterializePlan]].
- [[MaterializePlan]] can optionally use one [[GroupPlan]], which in turn uses [[GroupMode]].

## Related symbols

- [[TaskProvider]]
- [[TaskTemplate]]
- [[FoundTask]]
- [[TaskSet]]
- [[GroupPlan]]
- [[GroupMode]]
- [[MaterializePlan]]
