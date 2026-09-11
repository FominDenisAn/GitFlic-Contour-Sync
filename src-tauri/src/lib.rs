use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::error::Error as StdError;
use std::io::{Cursor, Read};
use std::time::Duration;
use url::Url;
use zip::ZipArchive;

fn is_en(locale: &str) -> bool {
    locale.eq_ignore_ascii_case("en")
}

fn normalize_url_impl(input: &str, locale: &str) -> Result<String, String> {
    let raw = input.trim();
    let value = if raw.contains("://") {
        raw.to_owned()
    } else {
        format!("https://{raw}")
    };

    let parsed = Url::parse(&value).map_err(|e| {
        if is_en(locale) {
            format!("Invalid URL: {e}")
        } else {
            format!("Некорректный URL: {e}")
        }
    })?;
    let host = parsed.host_str().ok_or_else(|| {
        if is_en(locale) {
            "The URL does not contain a host".to_string()
        } else {
            "В URL отсутствует host".to_string()
        }
    })?;
    let port = parsed.port().map(|p| format!(":{p}")).unwrap_or_default();
    Ok(format!("{}://{}{}", parsed.scheme(), host, port))
}

fn api_client(token: &str, locale: &str) -> Result<reqwest::Client, String> {
    if token.trim().is_empty() {
        return Err(if is_en(locale) {
            "API token is not specified".to_string()
        } else {
            "API-токен не указан".to_string()
        });
    }

    let mut headers = HeaderMap::new();
    let token_header = HeaderValue::from_str(&format!("token {}", token.trim())).map_err(|_| {
        if is_en(locale) {
            "Invalid API token".to_string()
        } else {
            "Некорректный API-токен".to_string()
        }
    })?;
    headers.insert(AUTHORIZATION, token_header);

    reqwest::Client::builder()
        .default_headers(headers)
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| {
            if is_en(locale) {
                format!("Could not create HTTP client: {e}")
            } else {
                format!("Не удалось создать HTTP-клиент: {e}")
            }
        })
}

fn request_error_details(error: &reqwest::Error, endpoint: &str, locale: &str) -> String {
    let category = if error.is_timeout() {
        if is_en(locale) { "Timeout" } else { "Тайм-аут" }
    } else if error.is_connect() {
        if is_en(locale) { "Connection" } else { "Подключение" }
    } else if error.is_request() {
        if is_en(locale) { "Request" } else { "Запрос" }
    } else if error.is_decode() {
        if is_en(locale) { "Response parsing" } else { "Разбор ответа" }
    } else if is_en(locale) {
        "Network"
    } else {
        "Сеть"
    };

    let mut lines = if is_en(locale) {
        vec![
            format!("Type: {category}"),
            format!("URL: {endpoint}"),
            format!("Error: {error}"),
        ]
    } else {
        vec![
            format!("Тип: {category}"),
            format!("URL: {endpoint}"),
            format!("Ошибка: {error}"),
        ]
    };

    let mut source = error.source();
    let mut depth = 0;
    while let Some(cause) = source {
        if is_en(locale) {
            lines.push(format!("Cause {}: {}", depth + 1, cause));
        } else {
            lines.push(format!("Причина {}: {}", depth + 1, cause));
        }
        source = cause.source();
        depth += 1;
        if depth >= 8 {
            break;
        }
    }
    lines.join("\n")
}

fn body_preview(body: &str, locale: &str) -> String {
    const LIMIT: usize = 1800;
    let clean = body.replace('\r', " ").replace('\0', "");
    if clean.chars().count() <= LIMIT {
        return clean;
    }
    let preview: String = clean.chars().take(LIMIT).collect();
    if is_en(locale) {
        format!("{preview}\n... response truncated ...")
    } else {
        format!("{preview}\n... ответ сокращён ...")
    }
}

fn http_error(status: reqwest::StatusCode, endpoint: &str, body: &str, locale: &str) -> String {
    let hint = if is_en(locale) {
        match status.as_u16() {
            400 => "Check the request parameters.",
            401 => "Check the API token.",
            403 => "The token does not have enough permissions for this request.",
            404 => "The method was not found on this GitFlic instance or the object is unavailable.",
            429 => "GitFlic rate-limited the request. Try again later.",
            500..=599 => "GitFlic returned a server-side error.",
            _ => "",
        }
    } else {
        match status.as_u16() {
            400 => "Проверьте параметры запроса.",
            401 => "Проверьте API-токен.",
            403 => "У токена недостаточно прав для этого запроса.",
            404 => "Метод не найден на этом GitFlic или объект недоступен.",
            429 => "GitFlic ограничил частоту запросов. Повторите позже.",
            500..=599 => "Ошибка на стороне GitFlic.",
            _ => "",
        }
    };

    if is_en(locale) {
        format!(
            "Type: HTTP\nURL: {endpoint}\nStatus: {status}\nHint: {hint}\nGitFlic response:\n{}",
            body_preview(body, locale)
        )
    } else {
        format!(
            "Тип: HTTP\nURL: {endpoint}\nСтатус: {status}\nПодсказка: {hint}\nОтвет GitFlic:\n{}",
            body_preview(body, locale)
        )
    }
}

async fn get_text(
    client: &reqwest::Client,
    endpoint: &str,
    locale: &str,
) -> Result<(reqwest::StatusCode, String), String> {
    let response = client
        .get(endpoint)
        .send()
        .await
        .map_err(|e| request_error_details(&e, endpoint, locale))?;
    let status = response.status();
    let body = response.text().await.map_err(|e| {
        if is_en(locale) {
            format!("Could not read the GitFlic response.\nURL: {endpoint}\nError: {e}")
        } else {
            format!("Не удалось прочитать ответ GitFlic.\nURL: {endpoint}\nОшибка: {e}")
        }
    })?;
    Ok((status, body))
}

async fn get_json(client: &reqwest::Client, endpoint: &str, locale: &str) -> Result<Value, String> {
    let (status, body) = get_text(client, endpoint, locale).await?;
    if !status.is_success() {
        return Err(http_error(status, endpoint, &body, locale));
    }

    serde_json::from_str(&body).map_err(|e| {
        if is_en(locale) {
            format!(
                "Type: Response parsing\nURL: {endpoint}\nJSON error: {e}\nGitFlic response:\n{}",
                body_preview(&body, locale)
            )
        } else {
            format!(
                "Тип: Разбор ответа\nURL: {endpoint}\nОшибка JSON: {e}\nОтвет GitFlic:\n{}",
                body_preview(&body, locale)
            )
        }
    })
}

async fn get_bytes(client: &reqwest::Client, endpoint: &str, locale: &str) -> Result<Vec<u8>, String> {
    let response = client
        .get(endpoint)
        .send()
        .await
        .map_err(|e| request_error_details(&e, endpoint, locale))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(http_error(status, endpoint, &body, locale));
    }
    response.bytes().await.map(|b| b.to_vec()).map_err(|e| {
        if is_en(locale) {
            format!("Could not download the GitFlic artifact: {e}")
        } else {
            format!("Не удалось скачать артефакт GitFlic: {e}")
        }
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserProfile {
    id: String,
    username: String,
    name: Option<String>,
    surname: Option<String>,
    full_name: Option<String>,
    email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectItem {
    id: String,
    owner: String,
    owner_type: Option<String>,
    name: String,
    alias: String,
    description: Option<String>,
    default_branch: Option<String>,
    work_branch: Option<String>,
    private: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BranchItem {
    name: String,
    sha: String,
    updated_at: Option<String>,
    default: bool,
    work: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthResult {
    base_url: String,
    user: UserProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncRequest {
    control_url: String,
    control_token: String,
    control_owner: String,
    control_project: String,
    control_branch: String,
    runner_tag: String,
    source_url: String,
    source_owner: String,
    source_project: String,
    source_branch: String,
    source_sha: String,
    target_url: String,
    target_owner: String,
    target_project: String,
    target_branch: String,
    target_sha: Option<String>,
    action: String,
    request_id: String,
    locale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PipelineJob {
    name: String,
    status: String,
    local_id: u64,
    stage_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiffLine {
    old_no: Option<u64>,
    new_no: Option<u64>,
    kind: String,
    text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiffFile {
    path: String,
    old_path: Option<String>,
    status: String,
    additions: u64,
    deletions: u64,
    binary: bool,
    lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunnerResult {
    request_id: String,
    pipeline_uuid: String,
    pipeline_local_id: u64,
    pipeline_status: String,
    action: String,
    state: String,
    source_sha: String,
    target_sha: Option<String>,
    actual_source_sha: Option<String>,
    actual_target_sha: Option<String>,
    post_target_sha: Option<String>,
    preflight_passed: bool,
    dry_run_passed: bool,
    changed_files: u64,
    additions: u64,
    deletions: u64,
    message: String,
    files: Vec<DiffFile>,
    commits: Vec<String>,
    jobs: Vec<PipelineJob>,
    truncated: bool,
}

fn project_from_value(value: &Value) -> Option<ProjectItem> {
    let id = value.get("id")?.as_str()?.to_string();
    let name = value.get("title")?.as_str()?.to_string();
    let alias = value.get("alias")?.as_str()?.to_string();
    let owner = value.get("owner")?.get("alias")?.as_str()?.to_string();

    Some(ProjectItem {
        id,
        owner,
        owner_type: value
            .get("owner")
            .and_then(|v| v.get("type"))
            .and_then(Value::as_str)
            .map(str::to_string),
        name,
        alias,
        description: value.get("description").and_then(Value::as_str).map(str::to_string),
        default_branch: value.get("defaultBranch").and_then(Value::as_str).map(str::to_string),
        work_branch: value.get("workBranch").and_then(Value::as_str).map(str::to_string),
        private: value.get("private").and_then(Value::as_bool).unwrap_or(false),
    })
}

fn branch_from_value(value: &Value) -> Option<BranchItem> {
    let name = value.get("name")?.as_str()?.to_string();
    let last_commit = value.get("lastCommit")?;
    let sha = last_commit.get("hash")?.as_str()?.to_string();

    Some(BranchItem {
        name,
        sha,
        updated_at: last_commit.get("createdAt").and_then(Value::as_str).map(str::to_string),
        default: value.get("default").and_then(Value::as_bool).unwrap_or(false),
        work: value.get("work").and_then(Value::as_bool).unwrap_or(false),
    })
}

async fn collect_projects_from_endpoint(
    client: &reqwest::Client,
    base: &str,
    path: &str,
    query: &str,
    locale: &str,
) -> Result<Vec<ProjectItem>, String> {
    let mut result = Vec::new();
    let mut page = 0u32;

    loop {
        let mut url = Url::parse(&format!("{base}/rest-api{path}")).map_err(|e| {
            if is_en(locale) { format!("Invalid API URL: {e}") } else { format!("Некорректный API URL: {e}") }
        })?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("page", &page.to_string());
            pairs.append_pair("size", "100");
            if !query.trim().is_empty() {
                pairs.append_pair("q", query.trim());
            }
        }

        let json = get_json(client, url.as_str(), locale).await?;
        if let Some(items) = json.get("_embedded").and_then(|v| v.get("projectList")).and_then(Value::as_array) {
            result.extend(items.iter().filter_map(project_from_value));
        }

        let total_pages = json.get("page").and_then(|v| v.get("totalPages")).and_then(Value::as_u64).unwrap_or(1);
        page += 1;
        if page as u64 >= total_pages || page >= 100 { break; }
    }

    Ok(result)
}

fn parse_env(text: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') { continue; }
        if let Some((key, value)) = line.split_once('=') {
            result.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    result
}

fn parse_bool(value: Option<&String>) -> bool {
    matches!(value.map(|s| s.as_str()), Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("YES"))
}

fn parse_u64(value: Option<&String>) -> u64 {
    value.and_then(|v| v.parse::<u64>().ok()).unwrap_or(0)
}

fn parse_numstat(text: &str) -> HashMap<String, (u64, u64, bool)> {
    let mut result = HashMap::new();
    for line in text.lines() {
        let mut parts = line.splitn(3, '\t');
        let add = parts.next().unwrap_or("0");
        let del = parts.next().unwrap_or("0");
        let path = parts.next().unwrap_or("").trim();
        if path.is_empty() { continue; }
        let binary = add == "-" || del == "-";
        result.insert(path.to_string(), (add.parse().unwrap_or(0), del.parse().unwrap_or(0), binary));
    }
    result
}

fn parse_name_status(text: &str) -> Vec<(String, Option<String>, String)> {
    let mut result = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 2 { continue; }
        let status_raw = parts[0];
        let status = status_raw.chars().next().unwrap_or('M').to_string();
        if status == "R" && parts.len() >= 3 {
            result.push((parts[2].to_string(), Some(parts[1].to_string()), status));
        } else {
            result.push((parts[1].to_string(), None, status));
        }
    }
    result
}

fn parse_patch(text: &str) -> (HashMap<String, Vec<DiffLine>>, HashMap<String, bool>, bool) {
    const MAX_LINES: usize = 20_000;
    let mut files: HashMap<String, Vec<DiffLine>> = HashMap::new();
    let mut binary: HashMap<String, bool> = HashMap::new();
    let mut current_path = String::new();
    let mut old_no: u64 = 0;
    let mut new_no: u64 = 0;
    let mut in_hunk = false;
    let mut total_lines = 0usize;
    let mut truncated = false;

    for line in text.lines() {
        if total_lines >= MAX_LINES {
            truncated = true;
            break;
        }
        if line.starts_with("diff --git ") {
            current_path.clear();
            in_hunk = false;
            continue;
        }
        if let Some(rest) = line.strip_prefix("+++ ") {
            if rest == "/dev/null" {
                continue;
            }
            current_path = rest.strip_prefix("b/").unwrap_or(rest).to_string();
            files.entry(current_path.clone()).or_default();
            continue;
        }
        if line.starts_with("Binary files ") || line.starts_with("GIT binary patch") {
            if !current_path.is_empty() {
                binary.insert(current_path.clone(), true);
            }
            continue;
        }
        if line.starts_with("@@ ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                old_no = parts[1].trim_start_matches('-').split(',').next().and_then(|v| v.parse().ok()).unwrap_or(0);
                new_no = parts[2].trim_start_matches('+').split(',').next().and_then(|v| v.parse().ok()).unwrap_or(0);
                in_hunk = true;
            }
            continue;
        }
        if !in_hunk || current_path.is_empty() { continue; }
        if line.starts_with("\\ No newline at end of file") { continue; }

        let (kind, old_value, new_value, content) = if let Some(content) = line.strip_prefix('+') {
            let n = new_no; new_no += 1; ("add", None, Some(n), content)
        } else if let Some(content) = line.strip_prefix('-') {
            let n = old_no; old_no += 1; ("remove", Some(n), None, content)
        } else if let Some(content) = line.strip_prefix(' ') {
            let o = old_no; let n = new_no; old_no += 1; new_no += 1; ("context", Some(o), Some(n), content)
        } else {
            continue;
        };

        files.entry(current_path.clone()).or_default().push(DiffLine {
            old_no: old_value,
            new_no: new_value,
            kind: kind.to_string(),
            text: content.to_string(),
        });
        total_lines += 1;
    }
    (files, binary, truncated)
}

fn parse_artifact_zip(bytes: Vec<u8>, locale: &str) -> Result<(HashMap<String, String>, String, String, String, String), String> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|e| {
        if is_en(locale) { format!("Could not open the runner artifact: {e}") } else { format!("Не удалось открыть артефакт раннера: {e}") }
    })?;

    let mut result_env = String::new();
    let mut patch = String::new();
    let mut name_status = String::new();
    let mut numstat = String::new();
    let mut commits = String::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name().replace('\\', "/");
        let mut content = String::new();
        if name.ends_with("result.env") || name.ends_with("changes.patch") || name.ends_with("name-status.txt") || name.ends_with("numstat.txt") || name.ends_with("commits.txt") {
            file.read_to_string(&mut content).map_err(|e| e.to_string())?;
        }
        if name.ends_with("result.env") { result_env = content; }
        else if name.ends_with("changes.patch") { patch = content; }
        else if name.ends_with("name-status.txt") { name_status = content; }
        else if name.ends_with("numstat.txt") { numstat = content; }
        else if name.ends_with("commits.txt") { commits = content; }
    }

    if result_env.is_empty() {
        return Err(if is_en(locale) {
            "The runner artifact does not contain result.env".to_string()
        } else {
            "В артефакте раннера отсутствует result.env".to_string()
        });
    }

    Ok((parse_env(&result_env), patch, name_status, numstat, commits))
}

fn build_runner_result(
    env: HashMap<String, String>,
    patch: String,
    name_status: String,
    numstat: String,
    commits_text: String,
    pipeline_uuid: String,
    pipeline_local_id: u64,
    pipeline_status: String,
    jobs: Vec<PipelineJob>,
) -> RunnerResult {
    let stats = parse_numstat(&numstat);
    let statuses = parse_name_status(&name_status);
    let (patch_lines, binary_map, truncated) = parse_patch(&patch);
    let mut files = Vec::new();

    for (path, old_path, status) in statuses {
        let (additions, deletions, binary_stat) = stats.get(&path).copied().unwrap_or((0, 0, false));
        files.push(DiffFile {
            path: path.clone(),
            old_path,
            status,
            additions,
            deletions,
            binary: binary_stat || binary_map.get(&path).copied().unwrap_or(false),
            lines: patch_lines.get(&path).cloned().unwrap_or_default(),
        });
    }

    // If name-status is empty but a patch exists, preserve visible files from the patch.
    if files.is_empty() {
        for (path, lines) in patch_lines {
            let (additions, deletions, binary_stat) = stats.get(&path).copied().unwrap_or((0, 0, false));
            files.push(DiffFile {
                path: path.clone(), old_path: None, status: "M".to_string(), additions, deletions,
                binary: binary_stat || binary_map.get(&path).copied().unwrap_or(false), lines,
            });
        }
    }

    files.sort_by(|a, b| a.path.to_lowercase().cmp(&b.path.to_lowercase()));
    let commits = commits_text.lines().filter(|l| !l.trim().is_empty()).map(str::to_string).collect::<Vec<_>>();

    RunnerResult {
        request_id: env.get("REQUEST_ID").cloned().unwrap_or_default(),
        pipeline_uuid,
        pipeline_local_id,
        pipeline_status,
        action: env.get("ACTION").cloned().unwrap_or_default(),
        state: env.get("STATE").cloned().unwrap_or_else(|| "UNKNOWN".to_string()),
        source_sha: env.get("EXPECTED_SOURCE_SHA").cloned().unwrap_or_default(),
        target_sha: env.get("EXPECTED_TARGET_SHA").filter(|v| !v.is_empty()).cloned(),
        actual_source_sha: env.get("ACTUAL_SOURCE_SHA").filter(|v| !v.is_empty()).cloned(),
        actual_target_sha: env.get("ACTUAL_TARGET_SHA").filter(|v| !v.is_empty()).cloned(),
        post_target_sha: env.get("POST_TARGET_SHA").filter(|v| !v.is_empty()).cloned(),
        preflight_passed: parse_bool(env.get("PREFLIGHT_PASSED")),
        dry_run_passed: parse_bool(env.get("DRY_RUN_PASSED")),
        changed_files: parse_u64(env.get("CHANGED_FILES")),
        additions: parse_u64(env.get("ADDITIONS")),
        deletions: parse_u64(env.get("DELETIONS")),
        message: env.get("MESSAGE").cloned().unwrap_or_default(),
        files,
        commits,
        jobs,
        truncated,
    }
}

fn required(value: &str, field_ru: &str, field_en: &str, locale: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(if is_en(locale) { format!("{field_en} is required") } else { format!("Не заполнено поле: {field_ru}") })
    } else { Ok(()) }
}

fn pipeline_variables(request: &SyncRequest) -> Vec<Value> {
    let target_sha = request.target_sha.clone().unwrap_or_default();
    let pairs = [
        ("CONTOUR_SYNC", "1".to_string()),
        ("SYNC_ACTION", request.action.clone()),
        ("SYNC_REQUEST_ID", request.request_id.clone()),
        ("SYNC_LOCALE", request.locale.clone()),
        ("RUNNER_TAG", request.runner_tag.clone()),
        ("SOURCE_BASE_URL", request.source_url.clone()),
        ("SOURCE_OWNER", request.source_owner.clone()),
        ("SOURCE_PROJECT", request.source_project.clone()),
        ("SOURCE_BRANCH", request.source_branch.clone()),
        ("EXPECTED_SOURCE_SHA", request.source_sha.clone()),
        ("TARGET_BASE_URL", request.target_url.clone()),
        ("TARGET_OWNER", request.target_owner.clone()),
        ("TARGET_PROJECT", request.target_project.clone()),
        ("TARGET_BRANCH", request.target_branch.clone()),
        ("EXPECTED_TARGET_SHA", target_sha),
    ];
    pairs.into_iter().map(|(key, value)| serde_json::json!({"key": key, "value": value})).collect()
}

async fn start_pipeline(client: &reqwest::Client, base: &str, request: &SyncRequest) -> Result<String, String> {
    let endpoint = format!("{base}/rest-api/project/{}/{}/cicd/pipeline/start", request.control_owner, request.control_project);
    let body = serde_json::json!({
        "refName": request.control_branch,
        "isTag": false,
        "variables": pipeline_variables(request),
    });
    let response = client.post(&endpoint).header(CONTENT_TYPE, "application/json").json(&body).send().await
        .map_err(|e| request_error_details(&e, &endpoint, &request.locale))?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() { return Err(http_error(status, &endpoint, &text, &request.locale)); }
    let json: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}\n{}", if is_en(&request.locale) {"Invalid pipeline response"} else {"Некорректный ответ запуска pipeline"}, body_preview(&text, &request.locale)))?;
    json.get("pipeline_uuid").and_then(Value::as_str).map(str::to_string).ok_or_else(|| {
        if is_en(&request.locale) { "GitFlic did not return pipeline_uuid".to_string() } else { "GitFlic не вернул pipeline_uuid".to_string() }
    })
}

async fn locate_pipeline(client: &reqwest::Client, base: &str, request: &SyncRequest, uuid: &str) -> Result<Option<(u64, String)>, String> {
    for page in 0..10u32 {
        let endpoint = format!("{base}/rest-api/project/{}/{}/cicd/pipeline?page={page}&size=100", request.control_owner, request.control_project);
        let json = get_json(client, &endpoint, &request.locale).await?;
        if let Some(items) = json.get("_embedded").and_then(|v| v.get("restPipelineModelList")).and_then(Value::as_array) {
            for item in items {
                if item.get("id").and_then(Value::as_str) == Some(uuid) {
                    let local_id = item.get("localId").and_then(Value::as_u64).or_else(|| item.get("localId").and_then(Value::as_i64).map(|v| v as u64)).unwrap_or(0);
                    let status = item.get("status").and_then(Value::as_str).unwrap_or("CREATED").to_string();
                    return Ok(Some((local_id, status)));
                }
            }
        }
        let total_pages = json.get("page").and_then(|v| v.get("totalPages")).and_then(Value::as_u64).unwrap_or(1);
        if page as u64 + 1 >= total_pages { break; }
    }
    Ok(None)
}

fn terminal_status(status: &str) -> bool {
    matches!(status, "SUCCESS" | "FAILED" | "CANCELED" | "SKIPPED" | "WARNING")
}

async fn get_jobs(client: &reqwest::Client, base: &str, request: &SyncRequest, local_id: u64) -> Result<Vec<PipelineJob>, String> {
    let endpoint = format!("{base}/rest-api/project/{}/{}/cicd/pipeline/{local_id}/jobs?page=0&size=100", request.control_owner, request.control_project);
    let json = get_json(client, &endpoint, &request.locale).await?;
    let mut jobs = Vec::new();
    if let Some(items) = json.get("_embedded").and_then(|v| v.get("restPipelineJobModelList")).and_then(Value::as_array) {
        for item in items {
            jobs.push(PipelineJob {
                name: item.get("name").and_then(Value::as_str).unwrap_or("").to_string(),
                status: item.get("status").and_then(Value::as_str).unwrap_or("CREATED").to_string(),
                local_id: item.get("localId").and_then(Value::as_u64).unwrap_or(0),
                stage_name: item.get("stageName").and_then(Value::as_str).map(str::to_string),
            });
        }
    }
    Ok(jobs)
}

#[tauri::command]
fn normalize_contour_url(url: String, locale: String) -> Result<String, String> {
    normalize_url_impl(&url, &locale)
}

#[tauri::command]
async fn authenticate_contour(url: String, token: String, locale: String) -> Result<AuthResult, String> {
    let base = normalize_url_impl(&url, &locale)?;
    let client = api_client(&token, &locale)?;
    let endpoint = format!("{base}/rest-api/user/me");
    let json = get_json(&client, &endpoint, &locale).await?;

    let user: UserProfile = serde_json::from_value(json).map_err(|e| {
        if is_en(&locale) { format!("Could not parse the GitFlic user profile: {e}") } else { format!("Не удалось разобрать профиль пользователя GitFlic: {e}") }
    })?;

    Ok(AuthResult { base_url: base, user })
}

#[tauri::command]
async fn list_projects(url: String, token: String, query: String, locale: String) -> Result<Vec<ProjectItem>, String> {
    let base = normalize_url_impl(&url, &locale)?;
    let client = api_client(&token, &locale)?;
    let owned = collect_projects_from_endpoint(&client, &base, "/project/my", &query, &locale).await?;
    let shared = collect_projects_from_endpoint(&client, &base, "/project/shared", &query, &locale).await?;

    let mut projects = BTreeMap::<String, ProjectItem>::new();
    for project in owned.into_iter().chain(shared) { projects.insert(project.id.clone(), project); }
    let mut values: Vec<ProjectItem> = projects.into_values().collect();
    values.sort_by(|a, b| a.owner.to_lowercase().cmp(&b.owner.to_lowercase()).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(values)
}

#[tauri::command]
async fn list_branches(url: String, token: String, owner: String, project: String, locale: String) -> Result<Vec<BranchItem>, String> {
    let base = normalize_url_impl(&url, &locale)?;
    let client = api_client(&token, &locale)?;
    let mut result = Vec::new();
    let mut page = 0u32;

    loop {
        let mut endpoint = Url::parse(&format!("{base}/rest-api/project/{owner}/{project}/branch")).map_err(|e| {
            if is_en(&locale) { format!("Invalid API URL: {e}") } else { format!("Некорректный API URL: {e}") }
        })?;
        {
            let mut pairs = endpoint.query_pairs_mut();
            pairs.append_pair("page", &page.to_string());
            pairs.append_pair("size", "100");
        }

        let (status, body) = get_text(&client, endpoint.as_str(), &locale).await?;
        if status == reqwest::StatusCode::NOT_FOUND && body.contains("BranchNotFoundException") {
            return Ok(Vec::new());
        }
        if !status.is_success() { return Err(http_error(status, endpoint.as_str(), &body, &locale)); }
        let json: Value = serde_json::from_str(&body).map_err(|e| {
            if is_en(&locale) { format!("Could not parse branch response: {e}") } else { format!("Не удалось разобрать ответ со списком веток: {e}") }
        })?;

        if let Some(items) = json.get("_embedded").and_then(|v| v.get("branchList")).and_then(Value::as_array) {
            result.extend(items.iter().filter_map(branch_from_value));
        }
        let total_pages = json.get("page").and_then(|v| v.get("totalPages")).and_then(Value::as_u64).unwrap_or(1);
        page += 1;
        if page as u64 >= total_pages || page >= 100 { break; }
    }

    result.sort_by(|a, b| b.default.cmp(&a.default).then_with(|| b.work.cmp(&a.work)).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(result)
}

#[tauri::command]
async fn run_sync_pipeline(request: SyncRequest) -> Result<RunnerResult, String> {
    required(&request.control_owner, "владелец проекта переноса", "transfer project owner", &request.locale)?;
    required(&request.control_project, "проект переноса", "transfer project", &request.locale)?;
    required(&request.control_branch, "ветка pipeline", "pipeline branch", &request.locale)?;
    required(&request.runner_tag, "тег раннера", "runner tag", &request.locale)?;
    required(&request.source_owner, "проект контура 1", "Contour 1 project", &request.locale)?;
    required(&request.source_branch, "ветка контура 1", "Contour 1 branch", &request.locale)?;
    required(&request.source_sha, "SHA контура 1", "Contour 1 SHA", &request.locale)?;
    required(&request.target_owner, "проект контура 2", "Contour 2 project", &request.locale)?;
    required(&request.target_branch, "ветка контура 2", "Contour 2 branch", &request.locale)?;
    if !matches!(request.action.as_str(), "preview" | "push") {
        return Err(if is_en(&request.locale) { "Unsupported runner action".to_string() } else { "Неподдерживаемая операция раннера".to_string() });
    }

    let base = normalize_url_impl(&request.control_url, &request.locale)?;
    let client = api_client(&request.control_token, &request.locale)?;
    let uuid = start_pipeline(&client, &base, &request).await?;

    let mut local_id = 0u64;
    let mut status = "CREATED".to_string();
    let mut found = false;

    for _ in 0..300 {
        if let Some((id, current)) = locate_pipeline(&client, &base, &request, &uuid).await? {
            local_id = id;
            status = current;
            found = true;
            if terminal_status(&status) { break; }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    if !found || local_id == 0 {
        return Err(if is_en(&request.locale) {
            format!("Pipeline {uuid} was started, but its local ID could not be found")
        } else {
            format!("Pipeline {uuid} запущен, но не удалось определить его локальный номер")
        });
    }
    if !terminal_status(&status) {
        return Err(if is_en(&request.locale) {
            format!("Pipeline #{local_id} did not finish within 10 minutes (status: {status})")
        } else {
            format!("Pipeline #{local_id} не завершился за 10 минут (статус: {status})")
        });
    }

    let jobs = get_jobs(&client, &base, &request, local_id).await.unwrap_or_default();
    let artifact_endpoint = format!("{base}/rest-api/project/{}/{}/cicd/pipeline/{local_id}/artifacts-download", request.control_owner, request.control_project);
    let bytes = get_bytes(&client, &artifact_endpoint, &request.locale).await?;
    let (env, patch, name_status, numstat, commits) = parse_artifact_zip(bytes, &request.locale)?;
    Ok(build_runner_result(env, patch, name_status, numstat, commits, uuid, local_id, status, jobs))
}

#[tauri::command]
fn app_contract() -> Value {
    serde_json::json!({
        "version": "0.3.0",
        "real_api_authentication": true,
        "real_project_listing": true,
        "real_branch_listing": true,
        "runner_preview_enabled": true,
        "runner_push_enabled": true,
        "write_enabled": true,
        "sync_scope": "single_branch",
        "transfer": "gitflic_runner",
        "race_protection": "source_recheck_and_target_force_with_lease",
        "tokens_persisted": false,
        "locales": ["ru", "en"]
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            normalize_contour_url,
            authenticate_contour,
            list_projects,
            list_branches,
            run_sync_pipeline,
            app_contract
        ])
        .run(tauri::generate_context!())
        .expect("error while running GitFlic Contour Sync");
}
