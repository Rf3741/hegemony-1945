import re

with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/estados.ron') as f:
    text = f.read()

for m in re.finditer(r'id:"(usa_[^"]+)"', text):
    print(m.group(1))
