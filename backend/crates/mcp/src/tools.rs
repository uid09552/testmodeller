//! The tool catalogue: task-level aggregates of REST operations.
//!
//! The table in docs/specification/05-api.md#mcp-tools is the contract; keep
//! the two in step. Every id an argument supplies is parsed as a UUID before it
//! is put into a path, and every enum is checked against its allowed values, so
//! a tool can only reach the routes it names.

use std::collections::HashMap;
use std::time::Duration;

use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::{ApiRequest, Backend, Body, Method};

type Args = Map<String, Value>;

/// Result of running a tool.
pub(crate) enum ToolOutcome {
    /// No tool has that name.
    Unknown,
    /// Success.
    Done(Value),
    /// A failure the model should see (bad arguments, a refused call).
    Failed(String),
}

struct Fail(String);
type Out = Result<Value, Fail>;

/// Upper bound on pages fetched by a listing, so a tool cannot loop forever.
const MAX_PAGES: usize = 50;
/// How long `request_ai_proposals` waits for its job.
const JOB_WAIT: Duration = Duration::from_secs(30);
const JOB_POLL: Duration = Duration::from_secs(1);
/// Items a listing returns unless `limit` says otherwise, and the most it may ask for.
const DEFAULT_LIMIT: usize = 100;
const MAX_LIMIT: usize = 500;

fn limit_prop() -> Value {
    json!({ "type": "integer", "minimum": 1, "maximum": MAX_LIMIT, "default": DEFAULT_LIMIT,
            "description": "Most items to return; `truncated` says whether more exist" })
}

fn detail_prop() -> Value {
    json!({ "enum": ["concise", "full"], "default": "concise",
            "description": "concise: ids, names and status only; full: every field" })
}

struct Tool {
    name: &'static str,
    title: &'static str,
    description: &'static str,
    read_only: bool,
    /// Changes or removes existing data (only meaningful when not read-only).
    destructive: bool,
    /// Calling it again with the same arguments changes nothing more.
    idempotent: bool,
    schema: fn() -> Value,
    /// Shape of `structuredContent` (always an object; see `protocol::wrap`).
    output: fn() -> Value,
}

fn id_prop(what: &str) -> Value {
    json!({ "type": "string", "format": "uuid", "description": what })
}

const TOOLS: &[Tool] = &[
    Tool {
        name: "tm_list_projects",
        title: "List projects",
        description: "All projects of your tenant, each with its Component > Feature > Model tree. \
                      Use it first to find the ids the other tools take.",
        read_only: true,
        destructive: false,
        idempotent: true,
        schema: || {
            json!({ "type": "object", "properties": { "limit": limit_prop(), "detail": detail_prop() } })
        },
        output: || {
            json!({ "type": "object", "required": ["projects", "truncated"], "properties": {
            "projects": { "type": "array", "items": { "type": "object", "required": ["project", "tree"],
                "properties": { "project": { "type": "object" }, "tree": { "type": "object" } } } },
            "truncated": { "type": "boolean" } } })
        },
    },
    Tool {
        name: "tm_search",
        title: "Search a project",
        description: "Search a project's components, features, models and test cases by text.",
        read_only: true,
        destructive: false,
        idempotent: true,
        schema: || {
            json!({
                "type": "object",
                "required": ["projectId", "query"],
                "properties": {
                    "projectId": id_prop("Project to search"),
                    "query": { "type": "string", "minLength": 1 },
                    "types": { "type": "array", "items": { "enum": ["component", "feature", "model", "testCase"] } },
                    "limit": limit_prop()
                }
            })
        },
        output: || {
            json!({ "type": "object", "required": ["hits", "truncated"], "properties": {
                "hits": { "type": "array" }, "truncated": { "type": "boolean" } } })
        },
    },
    Tool {
        name: "tm_get_model",
        title: "Get a model",
        description: "A model's states and transitions, with its validation issues and its coverage.",
        read_only: true,
        destructive: false,
        idempotent: true,
        schema: || json!({ "type": "object", "required": ["modelId"], "properties": { "modelId": id_prop("Model") } }),
        output: || {
            json!({ "type": "object", "required": ["model"], "properties": {
            "model": { "type": "object" },
            "validation": { "type": "array", "description": "Absent for a read-only key" },
            "coverage": { "type": "object" } } })
        },
    },
    Tool {
        name: "tm_save_model",
        title: "Create or replace a model",
        description: "Give featureId to create a model in that feature, or modelId to replace an \
                      existing model's states and transitions (its layout and variables are kept). \
                      State ids may be short keys such as \"start\"; transitions refer to them in \
                      from/to, and new UUIDs are assigned. Exactly one state must be initial.",
        read_only: false,
        destructive: true,
        idempotent: false,
        schema: || {
            json!({
                "type": "object",
                "required": ["states", "transitions"],
                "properties": {
                    "featureId": id_prop("Feature to create the model in"),
                    "modelId": id_prop("Model to replace"),
                    "name": { "type": "string", "description": "Required when creating" },
                    "description": { "type": "string" },
                    "states": { "type": "array", "items": {
                        "type": "object", "required": ["id", "name", "kind"],
                        "properties": {
                            "id": { "type": "string", "description": "UUID or a short key" },
                            "name": { "type": "string" },
                            "kind": { "enum": ["initial", "normal", "final"] },
                            "description": { "type": "string" },
                            "requirements": { "type": "array", "items": { "type": "string" }, "description": "Backlog URLs or short keys the state covers" }
                        }
                    } },
                    "transitions": { "type": "array", "items": {
                        "type": "object", "required": ["from", "to", "event"],
                        "properties": {
                            "from": { "type": "string" }, "to": { "type": "string" },
                            "event": { "type": "string" }, "guard": { "type": "string" },
                            "action": { "type": "string" }, "expected": { "type": "string" },
                            "requirements": { "type": "array", "items": { "type": "string" }, "description": "Backlog URLs or short keys the transition covers" }
                        }
                    } }
                }
            })
        },
        output: || {
            json!({ "type": "object", "required": ["created", "model"], "properties": {
            "created": { "type": "boolean", "description": "false when an existing model was replaced" },
            "model": { "type": "object" } } })
        },
    },
    Tool {
        name: "tm_generate_test_cases",
        title: "Generate test cases",
        description: "Generate test cases from a model. Deterministic for model, criterion and seed. \
                      With save=true they are stored and assigned to the model.",
        read_only: false,
        destructive: false,
        idempotent: false,
        schema: || {
            json!({
                "type": "object",
                "required": ["modelId"],
                "properties": {
                    "modelId": id_prop("Model"),
                    "criterion": { "enum": ["state", "transition", "transition-pair", "bounded-paths", "requirement"], "default": "transition" },
                    "seed": { "type": "integer" },
                    "maxPathLength": { "type": "integer", "minimum": 1, "description": "Required for bounded-paths" },
                    "save": { "type": "boolean", "default": false }
                }
            })
        },
        output: || {
            json!({ "type": "object", "description": "The generation result: test cases and coverage" })
        },
    },
    Tool {
        name: "tm_list_test_cases",
        title: "List test cases",
        description: "Test cases of a feature, or those assigned to a model. Give featureId, modelId or both.",
        read_only: true,
        destructive: false,
        idempotent: true,
        schema: || {
            json!({ "type": "object", "properties": {
                "featureId": id_prop("Feature"), "modelId": id_prop("Model"),
                "limit": limit_prop(), "detail": detail_prop()
            } })
        },
        output: || {
            json!({ "type": "object", "required": ["testCases", "truncated"], "properties": {
                "testCases": { "type": "array" }, "truncated": { "type": "boolean" } } })
        },
    },
    Tool {
        name: "tm_export",
        title: "Export test cases",
        description: "Export a project (or one feature or component) as Gherkin, CSV, JSON or a Playwright test skeleton.",
        read_only: true,
        destructive: false,
        idempotent: true,
        schema: || {
            json!({
                "type": "object",
                "required": ["projectId", "format"],
                "properties": {
                    "projectId": id_prop("Project"),
                    "format": { "enum": ["gherkin", "csv", "json", "playwright"] },
                    "featureId": id_prop("Only this feature"),
                    "componentId": id_prop("Only this component")
                }
            })
        },
        output: || {
            json!({ "type": "object", "description": "JSON export as an object; Gherkin and CSV as text in result",
            "properties": { "result": { "type": "string" } } })
        },
    },
    Tool {
        name: "tm_import_test_results",
        title: "Import test results",
        description: "Import a JUnit XML or Cucumber JSON result file into a project. Results are matched \
                      to test cases by an @tm-<test case id> tag or by name. dryRun reports without recording.",
        read_only: false,
        destructive: false,
        idempotent: true,
        schema: || {
            json!({
                "type": "object",
                "required": ["projectId", "format", "content"],
                "properties": {
                    "projectId": id_prop("Project"),
                    "format": { "enum": ["junit", "cucumber"] },
                    "content": { "type": "string", "description": "The file's content" },
                    "dryRun": { "type": "boolean", "default": false }
                }
            })
        },
        output: || {
            json!({ "type": "object", "description": "Import report: matched, unmatched and recorded results" })
        },
    },
    Tool {
        name: "tm_get_coverage",
        title: "Get coverage",
        description: "Coverage of a model, a feature or a component. Give exactly one id.",
        read_only: true,
        destructive: false,
        idempotent: true,
        schema: || {
            json!({ "type": "object", "properties": {
                "modelId": id_prop("Model"), "featureId": id_prop("Feature"), "componentId": id_prop("Component")
            } })
        },
        output: || {
            json!({ "type": "object" })
        },
    },
    Tool {
        name: "tm_get_traceability",
        title: "Get traceability",
        description: "Backlog items traced to test cases, model elements and implementations, with the gaps.",
        read_only: true,
        destructive: false,
        idempotent: true,
        schema: || json!({ "type": "object", "required": ["projectId"], "properties": { "projectId": id_prop("Project") } }),
        output: || {
            json!({ "type": "object", "description": "Backlog items with their test cases, model elements and gaps" })
        },
    },
    Tool {
        name: "tm_get_threat_model",
        title: "Get the threat model",
        description: "A project's threat model with its summary, threats and controls.",
        read_only: true,
        destructive: false,
        idempotent: true,
        schema: || json!({ "type": "object", "required": ["projectId"], "properties": { "projectId": id_prop("Project") } }),
        output: || {
            json!({ "type": "object", "required": ["threatModel", "summary", "threats", "controls"], "properties": {
            "threatModel": { "type": "object" }, "summary": { "type": "object" },
            "threats": { "type": "array" },
                    "controls": { "type": "array" } } })
        },
    },
    Tool {
        name: "tm_request_ai_proposals",
        title: "Request AI proposals",
        description: "Ask TestModeller's AI for proposals for a feature. Waits up to 30 s and returns the \
                      pending proposals, or the jobId to follow with tm_get_ai_job if it is still running. \
                      Nothing is accepted: show them to the user, then use tm_decide_proposal.",
        read_only: false,
        destructive: false,
        idempotent: false,
        schema: || {
            json!({
                "type": "object",
                "required": ["featureId", "kind"],
                "properties": {
                    "featureId": id_prop("Feature"),
                    "kind": { "enum": ["model", "states-and-transitions", "test-cases", "feature-description"] },
                    "modelId": id_prop("Model, for model-scoped kinds"),
                    "prompt": { "type": "string", "maxLength": 8000 },
                    "count": { "type": "integer", "minimum": 1, "maximum": 20 }
                }
            })
        },
        output: || {
            json!({ "type": "object", "required": ["note"], "properties": {
            "proposals": { "type": "array", "description": "Pending proposals, once the job has finished" },
            "jobId": { "type": "string", "description": "The job, while it is still running; pass it to tm_get_ai_job" },
            "job": { "type": "object", "description": "The job, while it is still running" },
            "note": { "type": "string" } } })
        },
    },
    Tool {
        name: "tm_get_ai_job",
        title: "Get an AI job",
        description: "Status of an AI job started by tm_request_ai_proposals; once it has succeeded, its \
                      pending proposals. Nothing is accepted.",
        read_only: true,
        destructive: false,
        idempotent: true,
        schema: || {
            json!({ "type": "object", "required": ["jobId"], "properties": { "jobId": id_prop("Job") } })
        },
        output: || {
            json!({ "type": "object", "required": ["job", "note"], "properties": {
                "job": { "type": "object" },
                "proposals": { "type": "array", "description": "Pending proposals, once the job has succeeded" },
                "note": { "type": "string" } } })
        },
    },
    Tool {
        name: "tm_list_proposals",
        title: "List AI proposals",
        description: "AI proposals of a project, optionally by status.",
        read_only: true,
        destructive: false,
        idempotent: true,
        schema: || {
            json!({
                "type": "object",
                "required": ["projectId"],
                "properties": {
                    "projectId": id_prop("Project"),
                    "status": { "enum": ["pending", "accepted", "rejected"] },
                    "limit": limit_prop(),
                    "detail": detail_prop()
                }
            })
        },
        output: || {
            json!({ "type": "object", "required": ["proposals", "truncated"], "properties": {
                "proposals": { "type": "array" }, "truncated": { "type": "boolean" } } })
        },
    },
    Tool {
        name: "tm_decide_proposal",
        title: "Accept or reject an AI proposal",
        description: "Accept (optionally with an edited payload) or reject a pending AI proposal. \
                      Only call this when the user has decided.",
        read_only: false,
        destructive: true,
        idempotent: false,
        schema: || {
            json!({
                "type": "object",
                "required": ["proposalId", "decision"],
                "properties": {
                    "proposalId": id_prop("Proposal"),
                    "decision": { "enum": ["accept", "reject"] },
                    "payload": { "type": "object", "description": "Edited payload, accept only" }
                }
            })
        },
        output: || {
            json!({ "type": "object", "description": "The decided proposal" })
        },
    },
    Tool {
        name: "tm_get_project_status",
        title: "Project status",
        description: "One call for the state of a project: validation issues and coverage per model, \
                      traceability gaps, pending AI proposals and the test cases whose last result \
                      failed. Validation is left out for a read-only key.",
        read_only: true,
        destructive: false,
        idempotent: true,
        schema: || {
            json!({ "type": "object", "required": ["projectId"], "properties": {
                "projectId": id_prop("Project") } })
        },
        output: || {
            json!({ "type": "object", "required": ["models", "traceability", "pendingProposals", "failedTestCases"],
            "properties": {
                "models": { "type": "array", "description": "Per model: id, name, validation, coverage" },
                "modelsTruncated": { "type": "boolean" },
                "traceability": { "type": "object", "description": "The traceability summary" },
                "pendingProposals": { "type": "integer" },
                "failedTestCases": { "type": "array" } } })
        },
    },
    Tool {
        name: "tm_triage_failures",
        title: "Triage failed tests",
        description: "Maps test cases whose last result failed or errored onto the states and \
                      transitions they are assigned to, most failures first, so you can see which \
                      part of a model is failing. Deterministic; nothing is guessed.",
        read_only: true,
        destructive: false,
        idempotent: true,
        schema: || {
            json!({ "type": "object", "required": ["projectId"], "properties": {
                "projectId": id_prop("Project"), "limit": limit_prop() } })
        },
        output: || {
            json!({ "type": "object", "required": ["elements", "unassigned", "truncated"], "properties": {
                "elements": { "type": "array", "description": "Model elements with their failing test cases" },
                "unassigned": { "type": "array", "description": "Failing test cases assigned to no element" },
                "truncated": { "type": "boolean" } } })
        },
    },
    Tool {
        name: "tm_update_test_case",
        title: "Update a test case",
        description: "Change only the fields you give; the rest, including assignments and threat \
                      links, is kept. `steps` replaces all steps.",
        read_only: false,
        destructive: true,
        idempotent: true,
        schema: || {
            json!({ "type": "object", "required": ["testCaseId"], "properties": {
                "testCaseId": id_prop("Test case"),
                "name": { "type": "string" }, "description": { "type": "string" },
                "preconditions": { "type": "string" },
                "priority": { "enum": ["low", "medium", "high", "critical"] },
                "status": { "enum": ["draft", "ready", "obsolete"] },
                "tags": { "type": "array", "items": { "type": "string" } },
                "implementationUrl": { "type": "string" }, "backlogUrl": { "type": "string" },
                "steps": { "type": "array", "items": { "type": "object", "required": ["action", "expected"],
                    "properties": { "action": { "type": "string" }, "expected": { "type": "string" } } } }
            } })
        },
        output: || {
            json!({ "type": "object", "required": ["testCase"], "properties": { "testCase": { "type": "object" } } })
        },
    },
    Tool {
        name: "tm_patch_model",
        title: "Patch a model",
        description: "Add or remove states and transitions of an existing model without resending \
                      it. New states use short keys; transitions may refer to existing state ids or \
                      to new keys. Removing a state also removes its transitions.",
        read_only: false,
        destructive: true,
        idempotent: false,
        schema: || {
            json!({ "type": "object", "required": ["modelId"], "properties": {
                "modelId": id_prop("Model"),
                "name": { "type": "string" }, "description": { "type": "string" },
                "addStates": { "type": "array", "items": { "type": "object", "required": ["id", "name", "kind"],
                    "properties": { "id": { "type": "string", "description": "A short key" },
                        "name": { "type": "string" }, "kind": { "enum": ["normal", "final"] },
                        "description": { "type": "string" } } } },
                "addTransitions": { "type": "array", "items": { "type": "object", "required": ["from", "to", "event"],
                    "properties": { "from": { "type": "string" }, "to": { "type": "string" },
                        "event": { "type": "string" }, "guard": { "type": "string" },
                        "action": { "type": "string" }, "expected": { "type": "string" } } } },
                "removeStateIds": { "type": "array", "items": id_prop("State") },
                "removeTransitionIds": { "type": "array", "items": id_prop("Transition") }
            } })
        },
        output: || {
            json!({ "type": "object", "required": ["model", "removedTransitions"], "properties": {
                "model": { "type": "object" },
                "removedTransitions": { "type": "integer", "description": "Transitions dropped with their states" } } })
        },
    },
];

/// The `tools/list` entries.
pub fn catalogue() -> Vec<Value> {
    TOOLS
        .iter()
        .map(|t| {
            json!({
                "name": t.name,
                "title": t.title,
                "description": t.description,
                "inputSchema": (t.schema)(),
                "outputSchema": (t.output)(),
                "annotations": {
                    "title": t.title,
                    "readOnlyHint": t.read_only,
                    "destructiveHint": !t.read_only && t.destructive,
                    "idempotentHint": t.read_only || t.idempotent,
                    "openWorldHint": false
                }
            })
        })
        .collect()
}

pub(crate) async fn run(backend: &dyn Backend, name: &str, args: &Args) -> ToolOutcome {
    let out = match name {
        "tm_list_projects" => list_projects(backend, args).await,
        "tm_search" => search(backend, args).await,
        "tm_get_model" => get_model(backend, args).await,
        "tm_save_model" => save_model(backend, args).await,
        "tm_generate_test_cases" => generate(backend, args).await,
        "tm_list_test_cases" => list_test_cases(backend, args).await,
        "tm_export" => export(backend, args).await,
        "tm_import_test_results" => import_results(backend, args).await,
        "tm_get_coverage" => coverage(backend, args).await,
        "tm_get_traceability" => {
            let pid = id(args, "projectId");
            match pid {
                Ok(pid) => get(backend, format!("/projects/{pid}/traceability")).await,
                Err(e) => Err(e),
            }
        }
        "tm_get_threat_model" => threat_model(backend, args).await,
        "tm_request_ai_proposals" => request_proposals(backend, args).await,
        "tm_get_ai_job" => get_ai_job(backend, args).await,
        "tm_list_proposals" => list_proposals(backend, args).await,
        "tm_decide_proposal" => decide(backend, args).await,
        "tm_get_project_status" => project_status(backend, args).await,
        "tm_triage_failures" => triage_failures(backend, args).await,
        "tm_update_test_case" => update_test_case(backend, args).await,
        "tm_patch_model" => patch_model(backend, args).await,
        _ => return ToolOutcome::Unknown,
    };
    match out {
        Ok(v) => ToolOutcome::Done(v),
        Err(Fail(m)) => ToolOutcome::Failed(m),
    }
}

// ── argument helpers ────────────────────────────────────────────────────────

fn opt_id(args: &Args, key: &str) -> Result<Option<Uuid>, Fail> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => s
            .parse()
            .map(Some)
            .map_err(|_| Fail(format!("\"{key}\" must be a UUID"))),
        Some(_) => Err(Fail(format!("\"{key}\" must be a UUID string"))),
    }
}

fn id(args: &Args, key: &str) -> Result<Uuid, Fail> {
    opt_id(args, key)?.ok_or_else(|| Fail(format!("\"{key}\" is required")))
}

fn opt_str<'a>(args: &'a Args, key: &str) -> Result<Option<&'a str>, Fail> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        Some(_) => Err(Fail(format!("\"{key}\" must be a string"))),
    }
}

fn string<'a>(args: &'a Args, key: &str) -> Result<&'a str, Fail> {
    opt_str(args, key)?
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| Fail(format!("\"{key}\" is required")))
}

fn choice<'a>(
    args: &'a Args,
    key: &str,
    allowed: &[&'a str],
    default: Option<&'a str>,
) -> Result<&'a str, Fail> {
    match opt_str(args, key)? {
        Some(v) => allowed
            .iter()
            .find(|a| **a == v)
            .copied()
            .ok_or_else(|| Fail(format!("\"{key}\" must be one of {}", allowed.join(", ")))),
        None => default.ok_or_else(|| Fail(format!("\"{key}\" is required"))),
    }
}

fn opt_bool(args: &Args, key: &str) -> Result<Option<bool>, Fail> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(Fail(format!("\"{key}\" must be a boolean"))),
    }
}

fn opt_int(args: &Args, key: &str) -> Result<Option<i64>, Fail> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_i64()
            .map(Some)
            .ok_or_else(|| Fail(format!("\"{key}\" must be an integer"))),
    }
}

fn array<'a>(args: &'a Args, key: &str) -> Result<&'a Vec<Value>, Fail> {
    args.get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| Fail(format!("\"{key}\" must be an array")))
}

// ── REST helpers ────────────────────────────────────────────────────────────

fn method_name(m: Method) -> &'static str {
    match m {
        Method::Get => "GET",
        Method::Post => "POST",
        Method::Put => "PUT",
    }
}

/// Runs a call; a non-2xx answer becomes a failure carrying the problem detail.
async fn send(backend: &dyn Backend, request: ApiRequest) -> Out {
    let label = format!("{} {}", method_name(request.method), request.path);
    let response = backend.call(request).await;
    if response.ok() {
        return Ok(response.body);
    }
    let detail = response
        .body
        .get("detail")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| response.body.to_string());
    let hint = match response.status {
        403 => " (an API key with the Editor role is needed to change data)",
        _ => "",
    };
    Err(Fail(format!(
        "{label} failed with {}: {detail}{hint}",
        response.status
    )))
}

async fn get(backend: &dyn Backend, path: String) -> Out {
    send(backend, ApiRequest::new(Method::Get, path)).await
}

/// Follows `nextCursor` until `cap` items or the last page; also says
/// whether more items exist than were returned.
async fn all_pages(
    backend: &dyn Backend,
    request: ApiRequest,
    cap: usize,
) -> Result<(Vec<Value>, bool), Fail> {
    let mut items = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..MAX_PAGES {
        let mut page = request.clone().query("limit", "200");
        if let Some(c) = &cursor {
            page = page.query("cursor", c.clone());
        }
        let body = send(backend, page).await?;
        if let Some(list) = body.get("items").and_then(Value::as_array) {
            items.extend(list.iter().cloned());
        }
        cursor = body
            .get("nextCursor")
            .and_then(Value::as_str)
            .map(str::to_owned);
        if items.len() > cap {
            items.truncate(cap);
            return Ok((items, true));
        }
        if cursor.is_none() {
            return Ok((items, false));
        }
        if items.len() == cap {
            return Ok((items, true));
        }
    }
    Ok((items, cursor.is_some()))
}

/// `limit` (1..=MAX_LIMIT, default DEFAULT_LIMIT).
fn limit(args: &Args) -> Result<usize, Fail> {
    match opt_int(args, "limit")? {
        None => Ok(DEFAULT_LIMIT),
        Some(n) if (1..=MAX_LIMIT as i64).contains(&n) => Ok(n as usize),
        Some(_) => Err(Fail(format!("\"limit\" must be between 1 and {MAX_LIMIT}"))),
    }
}

/// Whether `detail` asks for every field (default: concise).
fn full(args: &Args) -> Result<bool, Fail> {
    Ok(choice(args, "detail", &["concise", "full"], Some("concise"))? == "full")
}

/// Keeps only `keys` of an object.
fn pick(v: &Value, keys: &[&str]) -> Value {
    let mut out = Map::new();
    for k in keys {
        if let Some(x) = v.get(*k).filter(|x| !x.is_null()) {
            out.insert((*k).to_owned(), x.clone());
        }
    }
    Value::Object(out)
}

/// The concise form of a test case: what identifies it and how it last ran.
fn concise_test_case(tc: &Value) -> Value {
    let mut out = pick(tc, &["id", "name", "status", "featureId", "threatIds"]);
    if let Some(status) = tc.pointer("/lastResult/status") {
        out["lastResult"] = status.clone();
    }
    out
}

/// Cuts `items` to `cap`, saying whether any were dropped.
fn capped(mut items: Vec<Value>, cap: usize) -> (Vec<Value>, bool) {
    let truncated = items.len() > cap;
    items.truncate(cap);
    (items, truncated)
}

fn uuid_of(value: &Value, key: &str) -> Result<Uuid, Fail> {
    value
        .get(key)
        .and_then(Value::as_str)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| Fail(format!("the API answered without a valid \"{key}\"")))
}

// ── tools ───────────────────────────────────────────────────────────────────

async fn list_projects(backend: &dyn Backend, args: &Args) -> Out {
    let (cap, full) = (limit(args)?, full(args)?);
    let (projects, truncated) =
        all_pages(backend, ApiRequest::new(Method::Get, "/projects"), cap).await?;
    let mut out = Vec::with_capacity(projects.len());
    for project in projects {
        let pid = uuid_of(&project, "id")?;
        let tree = get(backend, format!("/projects/{pid}/tree")).await?;
        let project = if full {
            project
        } else {
            pick(&project, &["id", "name"])
        };
        out.push(json!({ "project": project, "tree": tree }));
    }
    Ok(json!({ "projects": out, "truncated": truncated }))
}

async fn search(backend: &dyn Backend, args: &Args) -> Out {
    let pid = id(args, "projectId")?;
    let mut request = ApiRequest::new(Method::Get, format!("/projects/{pid}/search"))
        .query("q", string(args, "query")?);
    if let Some(types) = args.get("types").and_then(Value::as_array) {
        let allowed = ["component", "feature", "model", "testCase"];
        let mut list = Vec::new();
        for t in types {
            match t.as_str().filter(|t| allowed.contains(t)) {
                Some(t) => list.push(t),
                None => {
                    return Err(Fail(format!(
                        "\"types\" entries must be one of {}",
                        allowed.join(", ")
                    )))
                }
            }
        }
        if !list.is_empty() {
            request = request.query("types", list.join(","));
        }
    }
    let cap = limit(args)?;
    let hits = send(backend, request).await?;
    let (hits, truncated) = capped(hits.as_array().cloned().unwrap_or_default(), cap);
    Ok(json!({ "hits": hits, "truncated": truncated }))
}

async fn get_model(backend: &dyn Backend, args: &Args) -> Out {
    let mid = id(args, "modelId")?;
    let model = get(backend, format!("/models/{mid}")).await?;
    let mut out = json!({ "model": model });
    // Validation is a POST: a read-only key gets the model without it.
    if let Ok(v) = send(
        backend,
        ApiRequest::new(Method::Post, format!("/models/{mid}/validate")),
    )
    .await
    {
        out["validation"] = v;
    }
    if let Ok(c) = get(backend, format!("/models/{mid}/coverage")).await {
        out["coverage"] = c;
    }
    Ok(out)
}

/// Maps the caller's state keys to UUIDs and resolves transition endpoints.
fn graph(args: &Args) -> Result<(Vec<Value>, Vec<Value>), Fail> {
    let mut keys: HashMap<String, Uuid> = HashMap::new();
    let mut states = Vec::new();
    for (i, s) in array(args, "states")?.iter().enumerate() {
        let s = s
            .as_object()
            .ok_or_else(|| Fail(format!("states[{i}] must be an object")))?;
        let key = s
            .get("id")
            .and_then(Value::as_str)
            .filter(|k| !k.is_empty())
            .ok_or_else(|| Fail(format!("states[{i}].id is required")))?;
        let uuid = key.parse().unwrap_or_else(|_| Uuid::new_v4());
        if keys.insert(key.to_owned(), uuid).is_some() {
            return Err(Fail(format!("state id \"{key}\" is used twice")));
        }
        let mut state = json!({
            "id": uuid,
            "name": s.get("name").cloned().unwrap_or(Value::Null),
            "kind": s.get("kind").cloned().unwrap_or(Value::Null),
        });
        for field in ["description", "position", "requirements"] {
            if let Some(v) = s.get(field) {
                state[field] = v.clone();
            }
        }
        states.push(state);
    }
    let mut transitions = Vec::new();
    for (i, t) in array(args, "transitions")?.iter().enumerate() {
        let t = t
            .as_object()
            .ok_or_else(|| Fail(format!("transitions[{i}] must be an object")))?;
        let end = |field: &str| -> Result<Uuid, Fail> {
            let key = t.get(field).and_then(Value::as_str).unwrap_or_default();
            keys.get(key).copied().ok_or_else(|| {
                Fail(format!(
                    "transitions[{i}].{field} \"{key}\" is not the id of a state in \"states\""
                ))
            })
        };
        let mut transition = json!({
            "from": end("from")?,
            "to": end("to")?,
            "event": t.get("event").cloned().unwrap_or(Value::Null),
        });
        if let Some(tid) = t
            .get("id")
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<Uuid>().ok())
        {
            transition["id"] = json!(tid);
        }
        for field in ["guard", "action", "expected", "requirements"] {
            if let Some(v) = t.get(field) {
                transition[field] = v.clone();
            }
        }
        transitions.push(transition);
    }
    Ok((states, transitions))
}

async fn save_model(backend: &dyn Backend, args: &Args) -> Out {
    let (states, transitions) = graph(args)?;
    let description = opt_str(args, "description")?;
    match (opt_id(args, "featureId")?, opt_id(args, "modelId")?) {
        (Some(fid), None) => {
            let mut body = json!({
                "name": string(args, "name")?,
                "states": states,
                "transitions": transitions,
            });
            if let Some(d) = description {
                body["description"] = json!(d);
            }
            let model = send(
                backend,
                ApiRequest::new(Method::Post, format!("/features/{fid}/models")).json(body),
            )
            .await?;
            Ok(json!({ "created": true, "model": model }))
        }
        (None, Some(mid)) => {
            let current = get(backend, format!("/models/{mid}")).await?;
            let version = current
                .get("version")
                .and_then(Value::as_i64)
                .ok_or_else(|| Fail("the model has no version".into()))?;
            let name = match opt_str(args, "name")? {
                Some(n) if !n.trim().is_empty() => json!(n),
                _ => current.get("name").cloned().unwrap_or(Value::Null),
            };
            // `variables` would be cleared if omitted; `layout` is kept when omitted.
            let mut body = json!({
                "name": name,
                "variables": current.get("variables").cloned().unwrap_or(json!([])),
                "states": states,
                "transitions": transitions,
            });
            match description {
                Some(d) => body["description"] = json!(d),
                None => {
                    if let Some(d) = current.get("description").filter(|d| !d.is_null()) {
                        body["description"] = d.clone();
                    }
                }
            }
            if let Some(s) = current.get("status").filter(|s| !s.is_null()) {
                body["status"] = s.clone();
            }
            let mut request = ApiRequest::new(Method::Put, format!("/models/{mid}")).json(body);
            request.if_match = Some(version);
            let model = send(backend, request).await?;
            Ok(json!({ "created": false, "model": model }))
        }
        _ => Err(Fail(
            "give either featureId (to create) or modelId (to replace), not both".into(),
        )),
    }
}

async fn generate(backend: &dyn Backend, args: &Args) -> Out {
    let mid = id(args, "modelId")?;
    let criterion = choice(
        args,
        "criterion",
        &[
            "state",
            "transition",
            "transition-pair",
            "bounded-paths",
            "requirement",
        ],
        Some("transition"),
    )?;
    let mut body =
        json!({ "criterion": criterion, "save": opt_bool(args, "save")?.unwrap_or(false) });
    if let Some(seed) = opt_int(args, "seed")? {
        body["seed"] = json!(seed);
    }
    if let Some(n) = opt_int(args, "maxPathLength")? {
        body["maxPathLength"] = json!(n);
    }
    send(
        backend,
        ApiRequest::new(Method::Post, format!("/models/{mid}/generate")).json(body),
    )
    .await
}

async fn list_test_cases(backend: &dyn Backend, args: &Args) -> Out {
    let (cap, full) = (limit(args)?, full(args)?);
    let (cases, truncated) = match (opt_id(args, "featureId")?, opt_id(args, "modelId")?) {
        (Some(fid), model) => {
            let mut request = ApiRequest::new(Method::Get, format!("/features/{fid}/test-cases"));
            if let Some(mid) = model {
                request = request.query("modelId", mid.to_string());
            }
            all_pages(backend, request, cap).await?
        }
        (None, Some(mid)) => {
            let all = get(backend, format!("/models/{mid}/test-cases")).await?;
            capped(all.as_array().cloned().unwrap_or_default(), cap)
        }
        (None, None) => return Err(Fail("give featureId, modelId or both".into())),
    };
    let cases: Vec<Value> = if full {
        cases
    } else {
        cases.iter().map(concise_test_case).collect()
    };
    Ok(json!({ "testCases": cases, "truncated": truncated }))
}

async fn export(backend: &dyn Backend, args: &Args) -> Out {
    let pid = id(args, "projectId")?;
    let format = choice(
        args,
        "format",
        &["gherkin", "csv", "json", "playwright"],
        None,
    )?;
    let mut request =
        ApiRequest::new(Method::Get, format!("/projects/{pid}/export")).query("format", format);
    if let Some(f) = opt_id(args, "featureId")? {
        request = request.query("featureId", f.to_string());
    }
    if let Some(c) = opt_id(args, "componentId")? {
        request = request.query("componentId", c.to_string());
    }
    send(backend, request).await
}

async fn import_results(backend: &dyn Backend, args: &Args) -> Out {
    let pid = id(args, "projectId")?;
    let format = choice(args, "format", &["junit", "cucumber"], None)?;
    let content = string(args, "content")?;
    let mut request = ApiRequest::new(Method::Post, format!("/projects/{pid}/test-results"))
        .query("format", format)
        .query(
            "dryRun",
            opt_bool(args, "dryRun")?.unwrap_or(false).to_string(),
        );
    request.body = Body::Text {
        content_type: if format == "junit" {
            "application/xml"
        } else {
            "application/json"
        },
        text: content.to_owned(),
    };
    send(backend, request).await
}

async fn coverage(backend: &dyn Backend, args: &Args) -> Out {
    let given = [
        ("models", opt_id(args, "modelId")?),
        ("features", opt_id(args, "featureId")?),
        ("components", opt_id(args, "componentId")?),
    ];
    let mut chosen = given.iter().filter_map(|(c, id)| id.map(|id| (c, id)));
    match (chosen.next(), chosen.next()) {
        (Some((collection, id)), None) => {
            get(backend, format!("/{collection}/{id}/coverage")).await
        }
        _ => Err(Fail(
            "give exactly one of modelId, featureId, componentId".into(),
        )),
    }
}

async fn threat_model(backend: &dyn Backend, args: &Args) -> Out {
    let pid = id(args, "projectId")?;
    let tm = get(backend, format!("/projects/{pid}/threat-model")).await?;
    let tid = uuid_of(&tm, "id")?;
    let summary = get(backend, format!("/threat-models/{tid}/summary")).await?;
    let threats = get(backend, format!("/threat-models/{tid}/threats")).await?;
    let controls = get(backend, format!("/threat-models/{tid}/controls")).await?;
    Ok(json!({ "threatModel": tm, "summary": summary, "threats": threats, "controls": controls }))
}

async fn request_proposals(backend: &dyn Backend, args: &Args) -> Out {
    let mut body = json!({
        "featureId": id(args, "featureId")?,
        "kind": choice(args, "kind", &["model", "states-and-transitions", "test-cases", "feature-description"], None)?,
    });
    if let Some(m) = opt_id(args, "modelId")? {
        body["modelId"] = json!(m);
    }
    if let Some(p) = opt_str(args, "prompt")? {
        body["prompt"] = json!(p);
    }
    if let Some(c) = opt_int(args, "count")? {
        body["count"] = json!(c);
    }
    let mut job = send(
        backend,
        ApiRequest::new(Method::Post, "/ai/proposals").json(body),
    )
    .await?;
    let jid = uuid_of(&job, "id")?;

    let mut waited = Duration::ZERO;
    loop {
        match job.get("status").and_then(Value::as_str) {
            Some("succeeded") => break,
            Some("failed") => {
                let detail = job
                    .pointer("/error/detail")
                    .and_then(Value::as_str)
                    .unwrap_or("no detail");
                return Err(Fail(format!("the AI job failed: {detail}")));
            }
            _ if waited >= JOB_WAIT => {
                return Ok(json!({
                    "jobId": jid,
                    "job": job,
                    "note": "The job is still running. Call tm_get_ai_job with this jobId in a moment."
                }))
            }
            _ => {}
        }
        tokio::time::sleep(JOB_POLL).await;
        waited += JOB_POLL;
        job = get(backend, format!("/jobs/{jid}")).await?;
    }

    Ok(json!({
        "proposals": proposals_of(backend, &job).await?,
        "note": PENDING_NOTE
    }))
}

const PENDING_NOTE: &str =
    "These are pending. Nothing was changed. Ask the user before calling tm_decide_proposal.";

/// The proposals a finished job created.
async fn proposals_of(backend: &dyn Backend, job: &Value) -> Result<Vec<Value>, Fail> {
    let mut proposals = Vec::new();
    for pid in job
        .get("proposalIds")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter_map(|s| s.parse::<Uuid>().ok())
    {
        proposals.push(get(backend, format!("/proposals/{pid}")).await?);
    }
    Ok(proposals)
}

async fn get_ai_job(backend: &dyn Backend, args: &Args) -> Out {
    let jid = id(args, "jobId")?;
    let job = get(backend, format!("/jobs/{jid}")).await?;
    match job.get("status").and_then(Value::as_str) {
        Some("succeeded") => Ok(json!({
            "proposals": proposals_of(backend, &job).await?,
            "job": job,
            "note": PENDING_NOTE
        })),
        Some("failed") => {
            let detail = job
                .pointer("/error/detail")
                .and_then(Value::as_str)
                .unwrap_or("no detail");
            Err(Fail(format!("the AI job failed: {detail}")))
        }
        _ => Ok(json!({
            "job": job,
            "note": "The job is still running. Call tm_get_ai_job again in a moment."
        })),
    }
}

async fn list_proposals(backend: &dyn Backend, args: &Args) -> Out {
    let pid = id(args, "projectId")?;
    let mut request = ApiRequest::new(Method::Get, format!("/projects/{pid}/proposals"));
    if args.get("status").is_some_and(|s| !s.is_null()) {
        request = request.query(
            "status",
            choice(args, "status", &["pending", "accepted", "rejected"], None)?,
        );
    }
    let (cap, full) = (limit(args)?, full(args)?);
    let (proposals, truncated) = all_pages(backend, request, cap).await?;
    let proposals: Vec<Value> = if full {
        proposals
    } else {
        proposals
            .iter()
            .map(|p| {
                pick(
                    p,
                    &["id", "kind", "status", "featureId", "modelId", "createdAt"],
                )
            })
            .collect()
    };
    Ok(json!({ "proposals": proposals, "truncated": truncated }))
}

async fn decide(backend: &dyn Backend, args: &Args) -> Out {
    let pid = id(args, "proposalId")?;
    let decision = choice(args, "decision", &["accept", "reject"], None)?;
    let mut request = ApiRequest::new(Method::Post, format!("/proposals/{pid}/{decision}"));
    match args.get("payload") {
        None | Some(Value::Null) => {}
        Some(Value::Object(p)) if decision == "accept" => {
            request = request.json(json!({ "payload": p }));
        }
        Some(Value::Object(_)) => {
            return Err(Fail("\"payload\" is only allowed with accept".into()))
        }
        Some(_) => return Err(Fail("\"payload\" must be an object".into())),
    }
    send(backend, request).await
}

// ── project status, triage, partial updates ─────────────────────────────────

/// Most models `tm_get_project_status` and `tm_triage_failures` look into.
const MAX_STATUS_MODELS: usize = 50;

/// Ids of every model in a project tree, in tree order.
fn model_ids(tree: &Value, out: &mut Vec<Uuid>) {
    match tree {
        Value::Object(map) => {
            if let Some(models) = map.get("models").and_then(Value::as_array) {
                out.extend(
                    models
                        .iter()
                        .filter_map(|m| m.get("id")?.as_str()?.parse::<Uuid>().ok()),
                );
            }
            map.iter()
                .filter(|(k, _)| k.as_str() != "models")
                .for_each(|(_, v)| model_ids(v, out));
        }
        Value::Array(items) => items.iter().for_each(|v| model_ids(v, out)),
        _ => {}
    }
}

/// Every test case of a traceability answer, linked or not.
fn trace_cases(trace: &Value) -> Vec<&Value> {
    let linked = trace
        .get("items")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|i| {
            i.get("testCases")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        });
    let untraced = trace
        .get("untraced")
        .and_then(Value::as_array)
        .into_iter()
        .flatten();
    linked.chain(untraced).collect()
}

fn has_failed(tc: &Value) -> bool {
    matches!(
        tc.pointer("/lastResult/status").and_then(Value::as_str),
        Some("failed" | "error")
    )
}

fn failure_of(tc: &Value) -> Value {
    json!({
        "id": tc.get("id"),
        "name": tc.get("name"),
        "status": tc.pointer("/lastResult/status"),
        "message": tc.pointer("/lastResult/message"),
        "runId": tc.pointer("/lastResult/runId"),
    })
}

async fn project_status(backend: &dyn Backend, args: &Args) -> Out {
    let pid = id(args, "projectId")?;
    let tree = get(backend, format!("/projects/{pid}/tree")).await?;
    let mut ids = Vec::new();
    model_ids(&tree, &mut ids);
    let models_truncated = ids.len() > MAX_STATUS_MODELS;
    ids.truncate(MAX_STATUS_MODELS);
    let mut models = Vec::with_capacity(ids.len());
    for mid in ids {
        let model = get(backend, format!("/models/{mid}")).await?;
        let mut entry = json!({ "id": mid, "name": model.get("name") });
        // Validation is a POST: a read-only key gets the model without it.
        if let Ok(v) = send(
            backend,
            ApiRequest::new(Method::Post, format!("/models/{mid}/validate")),
        )
        .await
        {
            entry["validation"] = v;
        }
        if let Ok(c) = get(backend, format!("/models/{mid}/coverage")).await {
            entry["coverage"] = c;
        }
        models.push(entry);
    }
    let trace = get(backend, format!("/projects/{pid}/traceability")).await?;
    let (pending, _) = all_pages(
        backend,
        ApiRequest::new(Method::Get, format!("/projects/{pid}/proposals"))
            .query("status", "pending"),
        MAX_LIMIT,
    )
    .await?;
    let failed: Vec<Value> = trace_cases(&trace)
        .into_iter()
        .filter(|tc| has_failed(tc))
        .map(failure_of)
        .collect();
    let (failed, _) = capped(failed, DEFAULT_LIMIT);
    Ok(json!({
        "models": models,
        "modelsTruncated": models_truncated,
        "traceability": trace.get("summary").cloned().unwrap_or(Value::Null),
        "pendingProposals": pending.len(),
        "failedTestCases": failed,
    }))
}

/// A readable name of a state or transition of `model`.
fn element_label(model: &Value, kind: &str, element: &str) -> Option<String> {
    let states = model.get("states")?.as_array()?;
    let name_of = |sid: &str| {
        states
            .iter()
            .find(|s| s.get("id").and_then(Value::as_str) == Some(sid))
            .and_then(|s| s.get("name")?.as_str())
            .unwrap_or("?")
            .to_owned()
    };
    if kind == "state" {
        return Some(name_of(element));
    }
    let t = model
        .get("transitions")?
        .as_array()?
        .iter()
        .find(|t| t.get("id").and_then(Value::as_str) == Some(element))?;
    let end = |f: &str| name_of(t.get(f).and_then(Value::as_str).unwrap_or_default());
    Some(format!(
        "{} -> {} on \"{}\"",
        end("from"),
        end("to"),
        t.get("event").and_then(Value::as_str).unwrap_or_default()
    ))
}

async fn triage_failures(backend: &dyn Backend, args: &Args) -> Out {
    let pid = id(args, "projectId")?;
    let cap = limit(args)?;
    let trace = get(backend, format!("/projects/{pid}/traceability")).await?;
    // (modelId, kind, elementId) -> failing test cases.
    let mut groups: std::collections::BTreeMap<(String, &'static str, String), Vec<Value>> =
        std::collections::BTreeMap::new();
    let mut unassigned = Vec::new();
    for tc in trace_cases(&trace).into_iter().filter(|tc| has_failed(tc)) {
        let elements = tc.get("elements").and_then(Value::as_array);
        let mut mapped = false;
        for e in elements.into_iter().flatten() {
            let Some(model) = e.get("modelId").and_then(Value::as_str) else {
                continue;
            };
            let (kind, element) = match (
                e.get("stateId").and_then(Value::as_str),
                e.get("transitionId").and_then(Value::as_str),
            ) {
                (_, Some(t)) => ("transition", t),
                (Some(s), None) => ("state", s),
                _ => continue,
            };
            groups
                .entry((model.to_owned(), kind, element.to_owned()))
                .or_default()
                .push(failure_of(tc));
            mapped = true;
        }
        if !mapped {
            unassigned.push(failure_of(tc));
        }
    }
    let mut ranked: Vec<_> = groups.into_iter().collect();
    ranked.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));
    let truncated = ranked.len() > cap;
    ranked.truncate(cap);
    let mut models: HashMap<String, Value> = HashMap::new();
    let mut elements = Vec::with_capacity(ranked.len());
    for ((model, kind, element), failures) in ranked {
        if !models.contains_key(&model) && models.len() < MAX_STATUS_MODELS {
            if let Ok(m) = get(backend, format!("/models/{}", uuid_str(&model)?)).await {
                models.insert(model.clone(), m);
            }
        }
        let m = models.get(&model);
        elements.push(json!({
            "modelId": model,
            "modelName": m.and_then(|m| m.get("name")),
            "type": kind,
            "id": element,
            "label": m.and_then(|m| element_label(m, kind, &element)),
            "failures": failures,
        }));
    }
    Ok(json!({ "elements": elements, "unassigned": unassigned, "truncated": truncated }))
}

/// Re-parses an id the API itself returned, so it is safe in a path.
fn uuid_str(s: &str) -> Result<Uuid, Fail> {
    s.parse()
        .map_err(|_| Fail("the API answered with an invalid id".into()))
}

async fn update_test_case(backend: &dyn Backend, args: &Args) -> Out {
    let tid = id(args, "testCaseId")?;
    let current = get(backend, format!("/test-cases/{tid}")).await?;
    let version = current
        .get("version")
        .and_then(Value::as_i64)
        .ok_or_else(|| Fail("the test case has no version".into()))?;
    let mut body = Map::new();
    // Start from what is stored, then apply the given fields.
    for field in [
        "name",
        "description",
        "preconditions",
        "priority",
        "status",
        "tags",
        "implementationUrl",
        "backlogUrl",
        "threatIds",
    ] {
        if let Some(v) = current.get(field).filter(|v| !v.is_null()) {
            body.insert(field.to_owned(), v.clone());
        }
    }
    for field in [
        "name",
        "description",
        "preconditions",
        "priority",
        "status",
        "tags",
        "implementationUrl",
        "backlogUrl",
    ] {
        if let Some(v) = args.get(field).filter(|v| !v.is_null()) {
            body.insert(field.to_owned(), v.clone());
        }
    }
    let steps = match args.get("steps") {
        Some(_) => array(args, "steps")?.clone(),
        None => current
            .get("steps")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|s| json!({ "action": s.get("action"), "expected": s.get("expected") }))
            .collect(),
    };
    body.insert("steps".into(), Value::Array(steps));
    let mut request =
        ApiRequest::new(Method::Put, format!("/test-cases/{tid}")).json(Value::Object(body));
    request.if_match = Some(version);
    let testcase = send(backend, request).await?;
    Ok(json!({ "testCase": testcase }))
}

async fn patch_model(backend: &dyn Backend, args: &Args) -> Out {
    let mid = id(args, "modelId")?;
    let current = get(backend, format!("/models/{mid}")).await?;
    let version = current
        .get("version")
        .and_then(Value::as_i64)
        .ok_or_else(|| Fail("the model has no version".into()))?;
    let removed_states: Vec<String> = id_list(args, "removeStateIds")?;
    let removed_transitions: Vec<String> = id_list(args, "removeTransitionIds")?;
    let mut states: Vec<Value> = current
        .get("states")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut transitions: Vec<Value> = current
        .get("transitions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let text = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).map(str::to_owned);
    states.retain(|s| !text(s, "id").is_some_and(|i| removed_states.contains(&i)));
    let before = transitions.len();
    transitions.retain(|t| {
        !text(t, "id").is_some_and(|i| removed_transitions.contains(&i))
            && !text(t, "from").is_some_and(|i| removed_states.contains(&i))
            && !text(t, "to").is_some_and(|i| removed_states.contains(&i))
    });
    let removed = before - transitions.len();

    // Existing states keep their ids; new ones are named by short keys.
    let mut keys: HashMap<String, Uuid> = states
        .iter()
        .filter_map(|s| {
            let id = text(s, "id")?;
            Some((id.clone(), id.parse().ok()?))
        })
        .collect();
    for (i, s) in optional_array(args, "addStates")?.iter().enumerate() {
        let key = s
            .get("id")
            .and_then(Value::as_str)
            .filter(|k| !k.is_empty())
            .ok_or_else(|| Fail(format!("addStates[{i}].id is required")))?;
        let uuid = Uuid::new_v4();
        if keys.insert(key.to_owned(), uuid).is_some() {
            return Err(Fail(format!("state id \"{key}\" is already in use")));
        }
        let mut state = json!({ "id": uuid, "name": s.get("name"), "kind": s.get("kind") });
        if let Some(d) = s.get("description") {
            state["description"] = d.clone();
        }
        states.push(state);
    }
    for (i, t) in optional_array(args, "addTransitions")?.iter().enumerate() {
        let end = |field: &str| -> Result<Uuid, Fail> {
            let key = t.get(field).and_then(Value::as_str).unwrap_or_default();
            keys.get(key).copied().ok_or_else(|| {
                Fail(format!(
                    "addTransitions[{i}].{field} \"{key}\" is neither an existing state nor in addStates"
                ))
            })
        };
        let mut transition = json!({ "id": Uuid::new_v4(), "from": end("from")?, "to": end("to")?, "event": t.get("event") });
        for field in ["guard", "action", "expected", "requirements"] {
            if let Some(v) = t.get(field) {
                transition[field] = v.clone();
            }
        }
        transitions.push(transition);
    }
    let name = match opt_str(args, "name")? {
        Some(n) if !n.trim().is_empty() => json!(n),
        _ => current.get("name").cloned().unwrap_or(Value::Null),
    };
    let mut body = json!({
        "name": name,
        "variables": current.get("variables").cloned().unwrap_or(json!([])),
        "states": states,
        "transitions": transitions,
    });
    match opt_str(args, "description")? {
        Some(d) => body["description"] = json!(d),
        None => {
            if let Some(d) = current.get("description").filter(|d| !d.is_null()) {
                body["description"] = d.clone();
            }
        }
    }
    if let Some(s) = current.get("status").filter(|s| !s.is_null()) {
        body["status"] = s.clone();
    }
    let mut request = ApiRequest::new(Method::Put, format!("/models/{mid}")).json(body);
    request.if_match = Some(version);
    let model = send(backend, request).await?;
    Ok(json!({ "model": model, "removedTransitions": removed }))
}

/// An optional array of UUID strings, each checked.
fn id_list(args: &Args, key: &str) -> Result<Vec<String>, Fail> {
    optional_array(args, key)?
        .iter()
        .map(|v| {
            v.as_str()
                .and_then(|s| s.parse::<Uuid>().ok())
                .map(|u| u.to_string())
                .ok_or_else(|| Fail(format!("\"{key}\" must hold UUIDs")))
        })
        .collect()
}

fn optional_array<'a>(args: &'a Args, key: &str) -> Result<&'a [Value], Fail> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(&[]),
        Some(_) => array(args, key).map(Vec::as_slice),
    }
}
