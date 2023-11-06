"""
Clean unused translate entries
"""

import os
import json
import sys
import subprocess
import toml

my_path = os.path.dirname(sys.argv[0])

with open(f'{my_path}/../i18n.toml', 'r', encoding="utf-8") as f:
    d = f.read()
    d = toml.loads(d)
    lang = d['fallback_language']

with open(f'{my_path}/../Cargo.toml', 'r', encoding="utf-8") as f:
    d = f.read()
    d = toml.loads(d)
    crate_name = d['package']['name']

no_res = []
with open(f'{my_path}/../i18n/{lang}/{crate_name}.ftl', 'r', encoding="utf-8") as f:
    d = f.readlines()
    table = {}
    for i in d:
        i_split = i.split('=')
        if len(i_split) == 2:
            name = i_split[0].strip()
            s = i_split[1].strip()
            table[name] = s

    for k in table:
        output = subprocess.Popen(
            [
                "rg",
                "-e",
                f'fl!\\("{k}"',
                "--json",
                f'{my_path}/../src'
            ],
            stdout=subprocess.PIPE
        ).stdout.readlines()
        for i in output:
            d = json.loads(i)
            if d.get('data'):
                if d['data'].get('stats'):
                    if d['data']['stats'].get('matches') is not None:
                        if d['data']['stats']['matches'] == 0:
                            no_res.append(k)

        output = subprocess.Popen(
            [
                "rg",
                "-U",
                "-e",
                f'fl!\\(\\n\\s*"{k}"',
                "--json",
                f'{my_path}/../src'
            ],
            stdout=subprocess.PIPE,
        ).stdout.readlines()

        for i in output:
            d = json.loads(i)
            if d.get('data'):
                if d['data'].get('stats'):
                    if d['data']['stats'].get('matches') is not None:
                        if d['data']['stats']['matches'] != 0 and k in no_res:
                            no_res.remove(k)


for i in os.walk(f'{my_path}/../i18n'):
    (path, d, f) = i
    for j in f:
        if j.endswith(".ftl"):
            lines = []
            with open(f'{path}/{j}', 'r', encoding="utf-8") as f:
                lines = f.readlines()
                IS_SET = False
                for i, c in enumerate(lines):
                    if IS_SET and (c.startswith(' ') or c.startswith('\t')):
                        lines[i] = ''
                    if IS_SET and not c.startswith(' ') and not c.startswith('\t'):
                        IS_SET = False
                    if c.split('=')[0].strip() in no_res:
                        lines[i] = ''
                        IS_SET = True
                lines = [i for i in lines if i != '']
            with open(f'{path}/{j}', 'w', encoding="utf-8") as f:
                f.writelines(lines)

for i in no_res:
    print(i)
