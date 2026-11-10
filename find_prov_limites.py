import re

with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/provincias.ron') as f:
    text = f.read()

recursos_found = set()
for m in re.finditer(r'limites_recursos:\{([^}]+)\}', text):
    for rm in re.finditer(r'"([^"]+)":', m.group(1)):
        recursos_found.add(rm.group(1))

print("Actual resources in provincias.ron limites_recursos:", sorted(recursos_found))
