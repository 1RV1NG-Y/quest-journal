"""Vendor selected SVGs from the pinned @iconify-json/devicon 1.2.68 npm tarball.
Usage: npm pack @iconify-json/devicon@1.2.68 --pack-destination /tmp
       python3 scripts/vendor-quest-icons.py /tmp/iconify-json-devicon-1.2.68.tgz
"""
import json
import re
import sys
import tarfile
from pathlib import Path

selection = [
    ('dotnetcore', '.NET', 'dotnet dot net asp.net aspnet microsoft', False),
    ('java', 'Java', 'jvm programming', False),
    ('csharp', 'C#', 'csharp c sharp dotnet', False),
    ('python', 'Python', 'programming data science', False),
    ('javascript', 'JavaScript', 'js web', False),
    ('typescript', 'TypeScript', 'ts web', False),
    ('react', 'React', 'frontend web', False),
    ('svelte', 'Svelte', 'frontend web', False),
    ('vuejs', 'Vue', 'frontend web', False),
    ('angular', 'Angular', 'frontend web', False),
    ('nodejs', 'Node.js', 'node backend javascript', False),
    ('spring', 'Spring', 'java backend', False),
    ('cplusplus', 'C++', 'cpp cplusplus', False),
    ('go', 'Go', 'golang backend', False),
    ('rust', 'Rust', 'systems programming', True),
    ('kotlin', 'Kotlin', 'android jvm', False),
    ('swift', 'Swift', 'apple ios', False),
    ('html5', 'HTML', 'web markup', False),
    ('css3', 'CSS', 'web styling', False),
    ('postgresql', 'PostgreSQL', 'postgres database sql', False),
    ('mysql', 'MySQL', 'database sql', False),
    ('mongodb', 'MongoDB', 'database nosql', False),
    ('docker', 'Docker', 'containers devops', False),
    ('kubernetes', 'Kubernetes', 'k8s containers devops', False),
    ('git', 'Git', 'version control', False),
    ('github', 'GitHub', 'git repositories', True),
    ('linux', 'Linux', 'operating system', False),
    ('azure', 'Azure', 'microsoft cloud', False),
]
root = Path(__file__).resolve().parent.parent
with tarfile.open(sys.argv[1]) as tar:
    package = json.load(tar.extractfile('package/package.json'))
    assert package['version'] == '1.2.68'
    data = json.load(tar.extractfile('package/icons.json'))
output = root / 'apps/desktop/public/quest-icons'
output.mkdir(parents=True, exist_ok=True)
catalog = []
for name, label, keywords, light in selection:
    icon = data['icons'][name]
    body = icon['body']
    assert not any(marker in body.lower() for marker in ('<script', '<foreignobject', 'onload=', 'javascript:'))
    assert all(value.startswith('#') for value in re.findall(r'href="([^"]*)"', body))
    width, height = icon.get('width', data['width']), icon.get('height', data['height'])
    (output / f'{name}.svg').write_text(f'<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 {width} {height}">{body}</svg>\n')
    catalog.append(dict(id=f'devicon:{name}', label=label, keywords=keywords, category='Tech', src=f'/quest-icons/{name}.svg', light=light))
(root / 'apps/desktop/src/lib/tech-icons.json').write_text(json.dumps(catalog, indent=2)+'\n')
print(f'Vendored {len(catalog)} Devicon SVGs.')
