$j = Get-Content 'C:\Users\joss1\.claude\projects\C--projects-projects-rust--vfx-rs\7316c7b0-ce59-4f6f-b48d-bee8d6a9ee27\tool-results\mcp-github-get_file_contents-1767985210764.txt' -Raw | ConvertFrom-Json
$content = $j[0].text
$pattern = 'OIIO_API\s+(?:ImageBuf|bool|void|int|size_t|float|std::\w+<?\w*>?)\s+(\w+)\s*\('
$matches = [regex]::Matches($content, $pattern)
$funcs = $matches | ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique
Write-Output "Total functions: $($funcs.Count)"
$funcs
