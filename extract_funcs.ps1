$content = Get-Content 'C:\projects\projects.rust\_vfx-rs\oiio_imagebufalgo.h' -Raw

# Pattern for OIIO function declarations
$pattern = '(?:ImageBuf|bool|void|int|size_t|float|std::string|CompareResults|PixelStats|ROI)\s+(\w+)\s*\('

$matches = [regex]::Matches($content, $pattern)
$funcs = $matches | ForEach-Object { $_.Groups[1].Value } | Where-Object { 
    $_ -notmatch '^(if|for|while|switch|return|sizeof|OIIO|defined)$' -and
    $_ -notmatch '^[A-Z_]+$'
} | Sort-Object -Unique

Write-Output "=== OIIO ImageBufAlgo functions ($($funcs.Count) total) ==="
$funcs | ForEach-Object { Write-Output $_ }
