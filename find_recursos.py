import re

with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/estados.ron') as f:
    text = f.read()

recursos_found = set()
for m in re.finditer(r'recursos:\{([^}]+)\}', text):
    for rm in re.finditer(r'"([^"]+)":', m.group(1)):
        recursos_found.add(rm.group(1))

print("Actual resources in estados.ron:", sorted(recursos_found))
