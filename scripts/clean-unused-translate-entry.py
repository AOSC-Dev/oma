import toml
import subprocess
import json
import os

lang = ''
with open('../i18n.toml', 'r') as f:
    d = f.read()
    d = toml.loads(d)
    lang = d['fallback_language']

crate_name = ''
with open('../Cargo.toml', 'r') as f:
    d = f.read()
    d = toml.loads(d)
    crate_name = d['package']['name']

no_res = []
with open(f'../i18n/{lang}/{crate_name}.ftl', 'r') as f:
    d = f.readlines()
    table = {}
    for i in d:
        i_split = i.split('=')
        if len(i_split) == 2:
            name = i_split[0].strip()
            s = i_split[1].strip()
            table[name] = s
    
    for k in table.keys():
        output = subprocess.Popen(["rg", "-e", f'fl!\("{k}"', "--json", "../src"], stdout=subprocess.PIPE).stdout.readlines()
        for i in output:
            d = json.loads(i)
            if d.get('data'):
                if d['data'].get('stats'):
                    if d['data']['stats'].get('matches') is not None:
                        if d['data']['stats']['matches'] == 0:
                            no_res.append(k)


for i in os.walk("../i18n"):
    (path, d, f) = i
    for j in f:
        if j.endswith(".ftl"):
            lines = []
            with open(f'{path}/{j}', 'r') as f:
                lines = f.readlines()
                is_set = False
                for i, c in enumerate(lines):
                    if is_set and (c.startswith(' ') or c.startswith('\t')):
                        lines[i] = ''
                    if is_set and not c.startswith(' ') and not c.startswith('\t'):
                        is_set = False
                    if c.split('=')[0].strip() in no_res:
                        lines[i] = ''
                        is_set = True
                lines = [i for i in lines if i != '']
            with open(f'{path}/{j}', 'w') as f:
                f.writelines(lines)
