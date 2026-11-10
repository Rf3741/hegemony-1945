with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/estados.ron') as f:
    text = f.read()

import re
m = re.search(r'id:\s*"usa_massachusetts".*?terra_aravel:\s*([0-9]+)', text, re.DOTALL)
if m:
    print("Mass terra_aravel:", m.group(1))
else:
    print("Not found terra_aravel in Massachusetts")
