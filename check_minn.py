import test_redist
for p in test_redist.provs:
    if 'Minnesota' in p['nome']:
        print(p['id'], p['nome'], p['recursos'])
