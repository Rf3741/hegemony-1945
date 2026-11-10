import re

with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/provincias.ron') as f:
    for line in f:
        if 'Massachusetts' in line:
            levels = sum(int(m.group(1)) for m in re.finditer(r'nivel:([0-9]+)', line))
            print("Massachusetts building levels:", levels)
            for m in re.finditer(r'tipo_predio_id:"([^"]+)"[^)]*?nivel:([0-9]+)', line):
                if int(m.group(2)) > 100:
                    print(f"  {m.group(1):30}: {m.group(2)}")
