import re

with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/provincias.ron') as f:
    text = f.read()

count = 0
for m in re.finditer(r'nome:"([^"]+)"[^)]*?limites_recursos:\{([^}]+)\}', text):
    if 'pesca' in m.group(2):
        count += 1
        if count <= 5:
            print(f"  {m.group(1)}: {m.group(2)}")
print("Total provinces with pesca in limites_recursos:", count)
