import re

with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/provincias.ron') as f:
    text = f.read()

total_pesca = 0
for line in text.splitlines():
    if 'pais_id:"usa"' in line:
        m = re.search(r'limites_recursos:\{[^}]*?"pesca":([0-9]+)', line)
        if m:
            total_pesca += int(m.group(1))

print("Total pesca deposit in USA:", total_pesca)
