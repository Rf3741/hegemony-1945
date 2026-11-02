import re, pathlib, collections
keys = collections.Counter()
for f in pathlib.Path('assets/data/eventos').glob('*.ron'):
    for m in re.finditer(r'Modificador\(\s*id:\s*"([^"]+)"', f.read_text()):
        keys[m.group(1)] += 1
for k in sorted(keys):
    print(f'{keys[k]:4d}  {k}')
print('total unicos:', len(keys))
