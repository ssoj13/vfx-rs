try:
    import PyOpenColorIO as ocio
except ImportError:
    import opencolorio as ocio

t = ocio.CDLTransform()
print("Default style:", t.getStyle())
print()
print("Available styles:")
for s in dir(ocio):
    if "CDL" in s:
        print(f"  {s}")
