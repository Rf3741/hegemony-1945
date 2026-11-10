import re

with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/provincias.ron') as f:
    for line in f:
        if 'Califórnia' in line:
            levels = sum(int(m.group(1)) for m in re.finditer(r'nivel:([0-9]+)', line))
            print("California building levels:", levels)
            for m in re.finditer(r'tipo_predio_id:"([^"]+)"[^)]*?nivel:([0-9]+)', line):
                print(f"  {m.group(1):30}: {m.group(2)}")
