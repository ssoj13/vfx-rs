
set "VFX=..\target\release\vfx.exe"
%VFX% c owl.jpg owl.exr && %VFX% diff owl.jpg owl.exr