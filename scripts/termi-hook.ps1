# termi-hook.ps1 — Claude Code Stop/UserPromptSubmit hook handler
# stdin으로 JSON을 받아 %APPDATA%/termi/events/ 에 이벤트 파일로 기록한다.

$ErrorActionPreference = 'SilentlyContinue'
$MAX_RANDOM_SUFFIX = 32767

# stdin에서 JSON 읽기
$json = [Console]::In.ReadToEnd()
if (-not $json) { exit 0 }

# JSON 파싱
try {
    $data = $json | ConvertFrom-Json
} catch {
    exit 0
}

# stop_hook_active 체크 — 재진입 방지 (Stop 이벤트 전용)
if ($data.stop_hook_active -eq $true) { exit 0 }

# events 디렉토리 생성
$eventsDir = Join-Path $env:APPDATA 'termi\events'
if (-not (Test-Path $eventsDir)) {
    New-Item -ItemType Directory -Path $eventsDir -Force | Out-Null
}

# hook_event_name에 따라 파일명 프리픽스 결정
$prefix = if ($data.hook_event_name -eq 'UserPromptSubmit') { 'prompt' } else { 'stop' }
$timestamp = Get-Date -Format 'yyyyMMddHHmmss'
$random = Get-Random -Maximum $MAX_RANDOM_SUFFIX
$filename = "$prefix-$timestamp-$random.json"

# 이벤트 파일 기록 (UTF-8, BOM 없이)
$path = Join-Path $eventsDir $filename
[System.IO.File]::WriteAllText($path, $json, [System.Text.UTF8Encoding]::new($false))
exit 0
