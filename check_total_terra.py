import re

with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/provincias.ron') as f:
    text = f.read()

total_terra = 0
for line in text.splitlines():
    if 'pais_id:"usa"' in line:
        m = re.search(r'terra_aravel_total:([0-9]+)', line)
        if m:
            total_terra += int(m.group(1))

print("Total terra_aravel_total in USA:", total_terra)
