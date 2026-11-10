import re

with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/estados.ron') as f:
    text = f.read()

print("Recursos containing 'pesca':")
for m in re.finditer(r'id:"([^"]+)"[^)]*?recursos:\{([^}]+)\}', text):
    if 'pesca' in m.group(2):
        print(f"  {m.group(1)}: {m.group(2)}")
