use super::examples_parse::{
    extract_first_python_string, extract_keyword_string, extract_parenthesized_body,
};
use super::model::EmbeddedExampleSources;

#[derive(Debug)]
pub(super) struct ExampleSourceDoc {
    pub(super) scenario: Option<String>,
    pub(super) task_docs: Vec<TaskDoc>,
}

#[derive(Debug)]
pub(super) struct TaskDoc {
    pub(super) name: String,
    pub(super) doc: String,
}

pub(super) fn extract_example_source_docs(sources: &EmbeddedExampleSources) -> ExampleSourceDoc {
    let mut scenario = None;
    let mut task_docs = Vec::new();

    for source in sources.source_files {
        if scenario.is_none() {
            scenario = extract_example_scenario(source.body);
        }
        task_docs.extend(extract_task_docs(source.body));
    }

    ExampleSourceDoc {
        scenario,
        task_docs,
    }
}

fn extract_example_scenario(body: &str) -> Option<String> {
    for line in body.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("# Scenario:") {
            let scenario = rest.trim();
            if !scenario.is_empty() {
                return Some(scenario.to_string());
            }
        }
    }
    None
}

fn extract_task_docs(body: &str) -> Vec<TaskDoc> {
    let mut task_docs = Vec::new();
    let mut cursor = 0;

    while let Some(relative) = body[cursor..].find("task(") {
        let start = cursor + relative;
        let Some((call_body, consumed)) =
            extract_parenthesized_body(&body[start + "task(".len()..])
        else {
            break;
        };

        if let Some(name) = extract_first_python_string(call_body)
            && let Some(doc) = extract_keyword_string(call_body, "doc")
        {
            task_docs.push(TaskDoc { name, doc });
        }

        cursor = start + "task(".len() + consumed;
    }

    task_docs
}
