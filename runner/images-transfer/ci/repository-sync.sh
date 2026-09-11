#!/usr/bin/env bash
set -u

OUT_DIR="${CI_PROJECT_DIR:-$(pwd)}/contour-sync-artifacts"
WORK_DIR="${CI_PROJECT_DIR:-$(pwd)}/.contour-sync-work"
REPO_DIR="$WORK_DIR/repository.git"
ASKPASS="$WORK_DIR/git-askpass.sh"

mkdir -p "$OUT_DIR" "$WORK_DIR"
rm -rf "$REPO_DIR"
: > "$OUT_DIR/changes.patch"
: > "$OUT_DIR/name-status.txt"
: > "$OUT_DIR/numstat.txt"
: > "$OUT_DIR/commits.txt"

STATE="ERROR"
PREFLIGHT_PASSED=0
DRY_RUN_PASSED=0
ACTUAL_SOURCE_SHA=""
ACTUAL_TARGET_SHA=""
POST_TARGET_SHA=""
CHANGED_FILES=0
ADDITIONS=0
DELETIONS=0
MESSAGE=""

msg() {
  if [ "${SYNC_LOCALE:-ru}" = "en" ]; then printf "%s" "$2"; else printf "%s" "$1"; fi
}

write_result() {
  cat > "$OUT_DIR/result.env" <<RESULT
REQUEST_ID=${SYNC_REQUEST_ID:-}
ACTION=${SYNC_ACTION:-}
STATE=$STATE
EXPECTED_SOURCE_SHA=${EXPECTED_SOURCE_SHA:-}
EXPECTED_TARGET_SHA=${EXPECTED_TARGET_SHA:-}
ACTUAL_SOURCE_SHA=$ACTUAL_SOURCE_SHA
ACTUAL_TARGET_SHA=$ACTUAL_TARGET_SHA
POST_TARGET_SHA=$POST_TARGET_SHA
PREFLIGHT_PASSED=$PREFLIGHT_PASSED
DRY_RUN_PASSED=$DRY_RUN_PASSED
CHANGED_FILES=$CHANGED_FILES
ADDITIONS=$ADDITIONS
DELETIONS=$DELETIONS
MESSAGE=$MESSAGE
RESULT
}

finish_blocked() {
  STATE="$1"
  MESSAGE="$2"
  write_result
  exit 0
}

finish_error() {
  STATE="ERROR"
  MESSAGE="$1"
  write_result
  exit 0
}

required_var() {
  local name="$1"
  if [ -z "${!name:-}" ]; then
    finish_error "$(msg "Не задана обязательная CI/CD переменная: $name" "Missing required CI variable: $name")"
  fi
}

for var in SYNC_ACTION SYNC_REQUEST_ID RUNNER_TAG SOURCE_BASE_URL SOURCE_OWNER SOURCE_PROJECT SOURCE_BRANCH EXPECTED_SOURCE_SHA TARGET_BASE_URL TARGET_OWNER TARGET_PROJECT TARGET_BRANCH EXPECTED_TARGET_SHA; do
  required_var "$var"
done

if [ "$SYNC_ACTION" != "preview" ] && [ "$SYNC_ACTION" != "push" ]; then
  finish_error "$(msg "Неподдерживаемое действие синхронизации" "Unsupported sync action")"
fi

SOURCE_GIT_USERNAME="${SOURCE_GIT_USERNAME:-}"
SOURCE_GIT_PASSWORD="${SOURCE_GIT_PASSWORD:-}"
TARGET_GIT_USERNAME="${TARGET_GIT_USERNAME:-}"
TARGET_GIT_PASSWORD="${TARGET_GIT_PASSWORD:-}"
SOURCE_HTTP_PROXY="${SOURCE_HTTP_PROXY:-}"
TARGET_HTTP_PROXY="${TARGET_HTTP_PROXY:-}"

required_var SOURCE_GIT_USERNAME
required_var SOURCE_GIT_PASSWORD
required_var TARGET_GIT_USERNAME
required_var TARGET_GIT_PASSWORD

cat > "$ASKPASS" <<'ASKPASS_EOF'
#!/usr/bin/env bash
case "$1" in
  *sername*|*Username*) printf '%s\n' "${SYNC_GIT_USERNAME:-}" ;;
  *assword*|*Password*) printf '%s\n' "${SYNC_GIT_PASSWORD:-}" ;;
  *) printf '\n' ;;
esac
ASKPASS_EOF
chmod 700 "$ASKPASS"

strip_slash() { printf '%s' "${1%/}"; }
SOURCE_REPO_URL="$(strip_slash "$SOURCE_BASE_URL")/project/$SOURCE_OWNER/$SOURCE_PROJECT.git"
TARGET_REPO_URL="$(strip_slash "$TARGET_BASE_URL")/project/$TARGET_OWNER/$TARGET_PROJECT.git"

run_git() {
  local side="$1"; shift
  local username password proxy
  if [ "$side" = "source" ]; then
    username="$SOURCE_GIT_USERNAME"; password="$SOURCE_GIT_PASSWORD"; proxy="$SOURCE_HTTP_PROXY"
  else
    username="$TARGET_GIT_USERNAME"; password="$TARGET_GIT_PASSWORD"; proxy="$TARGET_HTTP_PROXY"
  fi
  if [ -n "$proxy" ]; then
    SYNC_GIT_USERNAME="$username" SYNC_GIT_PASSWORD="$password" GIT_ASKPASS="$ASKPASS" GIT_TERMINAL_PROMPT=0 \
      git -c "http.proxy=$proxy" "$@"
  else
    SYNC_GIT_USERNAME="$username" SYNC_GIT_PASSWORD="$password" GIT_ASKPASS="$ASKPASS" GIT_TERMINAL_PROMPT=0 \
      git "$@"
  fi
}

if ! git init --bare "$REPO_DIR" >/dev/null 2>&1; then
  finish_error "$(msg "Не удалось создать временный Git-репозиторий" "Could not initialize temporary repository")"
fi
cd "$REPO_DIR" || finish_error "$(msg "Не удалось открыть временный Git-репозиторий" "Could not enter temporary repository")"

if ! run_git source fetch --no-tags --force "$SOURCE_REPO_URL" "refs/heads/$SOURCE_BRANCH:refs/sync/source" >/dev/null 2>&1; then
  finish_error "$(msg "Не удалось получить выбранную ветку из Контура 1" "Could not fetch selected branch from Contour 1")"
fi
ACTUAL_SOURCE_SHA="$(git rev-parse refs/sync/source 2>/dev/null || true)"
if [ -z "$ACTUAL_SOURCE_SHA" ]; then
  finish_error "$(msg "Не удалось определить SHA выбранной ветки в Контуре 1" "Could not resolve selected branch in Contour 1")"
fi

if ! run_git target fetch --no-tags --force "$TARGET_REPO_URL" "refs/heads/$TARGET_BRANCH:refs/sync/target" >/dev/null 2>&1; then
  finish_error "$(msg "Не удалось получить выбранную ветку из Контура 2" "Could not fetch selected branch from Contour 2")"
fi
ACTUAL_TARGET_SHA="$(git rev-parse refs/sync/target 2>/dev/null || true)"
if [ -z "$ACTUAL_TARGET_SHA" ]; then
  finish_error "$(msg "Не удалось определить SHA выбранной ветки в Контуре 2" "Could not resolve selected branch in Contour 2")"
fi

if [ "$ACTUAL_SOURCE_SHA" != "$EXPECTED_SOURCE_SHA" ]; then
  finish_blocked "SOURCE_CHANGED" "$(msg "Контур 1 изменился после сравнения" "Contour 1 changed after comparison")"
fi
if [ "$ACTUAL_TARGET_SHA" != "$EXPECTED_TARGET_SHA" ]; then
  finish_blocked "TARGET_CHANGED" "$(msg "Контур 2 изменился после сравнения" "Contour 2 changed after comparison")"
fi

if [ "$ACTUAL_SOURCE_SHA" = "$ACTUAL_TARGET_SHA" ]; then
  STATE="SYNCED"
  PREFLIGHT_PASSED=1
  DRY_RUN_PASSED=1
  MESSAGE="$(msg "Выбранные ветки уже синхронизированы" "Selected branches are already synchronized")"
  POST_TARGET_SHA="$ACTUAL_TARGET_SHA"
  write_result
  exit 0
fi

if git merge-base --is-ancestor "$ACTUAL_TARGET_SHA" "$ACTUAL_SOURCE_SHA" >/dev/null 2>&1; then
  STATE="DEV_AHEAD"
elif git merge-base --is-ancestor "$ACTUAL_SOURCE_SHA" "$ACTUAL_TARGET_SHA" >/dev/null 2>&1; then
  finish_blocked "PROD_AHEAD" "$(msg "В Контуре 2 есть коммиты, которых нет в Контуре 1" "Contour 2 contains commits that are not present in Contour 1")"
else
  finish_blocked "DIVERGED" "$(msg "История выбранных веток разошлась" "Selected branches have diverged")"
fi

PREFLIGHT_PASSED=1

git diff --no-ext-diff --find-renames --unified=3 "$ACTUAL_TARGET_SHA" "$ACTUAL_SOURCE_SHA" > "$OUT_DIR/changes.patch" 2>/dev/null || true
git diff --find-renames --name-status "$ACTUAL_TARGET_SHA" "$ACTUAL_SOURCE_SHA" > "$OUT_DIR/name-status.txt" 2>/dev/null || true
git diff --find-renames --numstat "$ACTUAL_TARGET_SHA" "$ACTUAL_SOURCE_SHA" > "$OUT_DIR/numstat.txt" 2>/dev/null || true
git log --format='%h %s' "$ACTUAL_TARGET_SHA..$ACTUAL_SOURCE_SHA" > "$OUT_DIR/commits.txt" 2>/dev/null || true

CHANGED_FILES="$(awk 'NF {count++} END {print count+0}' "$OUT_DIR/name-status.txt")"
ADDITIONS="$(awk -F '\t' '$1 ~ /^[0-9]+$/ {sum += $1} END {print sum+0}' "$OUT_DIR/numstat.txt")"
DELETIONS="$(awk -F '\t' '$2 ~ /^[0-9]+$/ {sum += $2} END {print sum+0}' "$OUT_DIR/numstat.txt")"

if ! run_git target push --dry-run "$TARGET_REPO_URL" "refs/sync/source:refs/heads/$TARGET_BRANCH" >/dev/null 2>&1; then
  finish_blocked "ERROR" "$(msg "Проверочный Git push отклонил обновление выбранной ветки" "Git dry-run rejected the selected branch update")"
fi
DRY_RUN_PASSED=1

if [ "$SYNC_ACTION" = "preview" ]; then
  MESSAGE="$(msg "Проверка завершена, ветку можно обновить" "Verification completed; update can be performed")"
  write_result
  exit 0
fi

# Race protection: fetch both refs again immediately before the real push.
git update-ref -d refs/sync/source-latest >/dev/null 2>&1 || true
git update-ref -d refs/sync/target-latest >/dev/null 2>&1 || true
if ! run_git source fetch --no-tags --force "$SOURCE_REPO_URL" "refs/heads/$SOURCE_BRANCH:refs/sync/source-latest" >/dev/null 2>&1; then
  finish_error "$(msg "Не удалось повторно проверить Контур 1 перед записью" "Could not recheck Contour 1 before update")"
fi
if ! run_git target fetch --no-tags --force "$TARGET_REPO_URL" "refs/heads/$TARGET_BRANCH:refs/sync/target-latest" >/dev/null 2>&1; then
  finish_error "$(msg "Не удалось повторно проверить Контур 2 перед записью" "Could not recheck Contour 2 before update")"
fi

LATEST_SOURCE_SHA="$(git rev-parse refs/sync/source-latest 2>/dev/null || true)"
LATEST_TARGET_SHA="$(git rev-parse refs/sync/target-latest 2>/dev/null || true)"
if [ "$LATEST_SOURCE_SHA" != "$EXPECTED_SOURCE_SHA" ]; then
  ACTUAL_SOURCE_SHA="$LATEST_SOURCE_SHA"
  finish_blocked "SOURCE_CHANGED" "$(msg "Контур 1 изменился после проверки" "Contour 1 changed after verification")"
fi
if [ "$LATEST_TARGET_SHA" != "$EXPECTED_TARGET_SHA" ]; then
  ACTUAL_TARGET_SHA="$LATEST_TARGET_SHA"
  finish_blocked "TARGET_CHANGED" "$(msg "Контур 2 изменился после проверки" "Contour 2 changed after verification")"
fi

if ! git merge-base --is-ancestor "$LATEST_TARGET_SHA" "$LATEST_SOURCE_SHA" >/dev/null 2>&1; then
  finish_blocked "DIVERGED" "$(msg "Связь истории веток изменилась перед записью" "Branch relationship changed before update")"
fi

# The lease makes the remote update conditional on the exact target SHA verified above.
if ! run_git target push --force-with-lease="refs/heads/$TARGET_BRANCH:$EXPECTED_TARGET_SHA" "$TARGET_REPO_URL" "refs/sync/source-latest:refs/heads/$TARGET_BRANCH" >/dev/null 2>&1; then
  finish_blocked "TARGET_CHANGED" "$(msg "Контур 2 изменился до подтверждения записи" "Contour 2 changed before the update was accepted")"
fi

POST_TARGET_SHA="$(run_git target ls-remote "$TARGET_REPO_URL" "refs/heads/$TARGET_BRANCH" 2>/dev/null | awk 'NR==1 {print $1}')"
if [ "$POST_TARGET_SHA" != "$LATEST_SOURCE_SHA" ]; then
  finish_error "$(msg "Итоговая проверка после записи не пройдена" "Post-update verification failed")"
fi

ACTUAL_SOURCE_SHA="$LATEST_SOURCE_SHA"
ACTUAL_TARGET_SHA="$LATEST_TARGET_SHA"
STATE="SYNCED"
MESSAGE="$(msg "Выбранная ветка обновлена и проверена" "Selected branch updated and verified")"
write_result
exit 0
