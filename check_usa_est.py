with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/estados.ron') as f:
    text = f.read()

import re
for m in re.finditer(r'id:\s*"([^"]+)",\s*nome:\s*"([^"]+)",\s*pais_id:\s*"usa"', text):
    print(m.group(1), m.group(2))
