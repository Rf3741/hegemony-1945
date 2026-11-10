import test_redist
for p in test_redist.provs:
    if 'Iowa' in p['nome']:
        print(p['id'], p['nome'], p['predios'])
