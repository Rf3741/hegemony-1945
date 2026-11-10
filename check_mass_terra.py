import re

with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/estados.ron') as f:
    text = f.read()

for m in re.finditer(r'id:"([^"]+)"[^)]*?terra_aravel:([0-9]+)', text):
    if 'massachusetts' in m.group(1):
        print(f"  {m.group(1)}: {m.group(2)}")
