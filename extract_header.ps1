$json = Get-Content 'C:\Users\joss1\.claude\projects\C--projects-projects-rust--vfx-rs\7316c7b0-ce59-4f6f-b48d-bee8d6a9ee27\tool-results\mcp-fetch-fetch-1767985398102.txt' -Raw | ConvertFrom-Json
$content = $json[0].text
$content | Set-Content 'C:\projects\projects.rust\_vfx-rs\oiio_imagebufalgo.h' -Encoding UTF8
Write-Output "Saved $($content.Length) chars"
