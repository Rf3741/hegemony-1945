import os
import re

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

with open("assets/data/economia/potenciais.ron", "r", encoding="utf-8") as f:
    text = f.read()

usa_states = re.findall(r'estado_id:\s*"(usa_[^"]+)"', text)
print(f"USA states in potenciais.ron: {len(usa_states)}")
for m in re.finditer(r'estado_id:\s*"(usa_[^"]+)",\s*terra_aravel:\s*(\d+)', text):
    print(f"  {m.group(1)}: {m.group(2)}")
