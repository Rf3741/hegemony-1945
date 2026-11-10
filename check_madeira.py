import re

with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/estados.ron') as f:
    text = f.read()

count = 0
for m in re.finditer(r'id:"([^"]+)"[^)]*?recursos:\{([^}]+)\}', text):
    if 'madeira' in m.group(2):
        count += 1
print("Total states with madeira in recursos:", count)
