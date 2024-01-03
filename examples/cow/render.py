import bpy
import sys

if len(sys.argv) != 2:
	print("usage: python render.py <file.stl>")
	sys.exit(1)

for ob in bpy.data.scenes['Scene'].objects:
	ob.select_set(ob.type == 'MESH' and ob.name.startswith("Cube"))
bpy.ops.object.delete()
cow = bpy.ops.import_mesh.stl(filepath=sys.argv[1])
bpy.data.scenes['Scene'].render.filepath = f"{sys.argv[1]}.png"
bpy.ops.render.render(write_still=True)
