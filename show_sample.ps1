$content = Get-Content 'C:\projects\projects.rust\_vfx-rs\oiio_imagebufalgo.h' -Raw
Write-Output "Length: $($content.Length)"
Write-Output "Sample (4000-6000):"
Write-Output $content.Substring(4000, 2000)
