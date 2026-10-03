"""Prefix a board's CSS classes and keyframes so they cannot leak into imported components.

Classes the board's script hands out at runtime (state modifiers such as `sel`, `on`) keep their
name; every rule that uses them must also carry a prefixed class, which `check` verifies.
"""
import re, sys

def scope(html, prefix, keep=None):
    head, rest = html.split('<style>', 1)
    css, tail = rest.split('</style>', 1)
    body, script = tail.split('<script type="text/x-dc"', 1)
    script = '<script type="text/x-dc"' + script
    css_classes = set(re.findall(r'\.([a-zA-Z][\w-]*)', re.sub(r'url\([^)]*\)', '', css)))
    js_strings = re.findall(r"'([^'\\]*)'", script.split('data-props=', 1)[1])
    dynamic = {t for s in js_strings for t in s.split() if t in css_classes}
    if keep is not None:
        dynamic = set(keep) & css_classes
    rename = {c: prefix + c for c in css_classes if c not in dynamic and not c.startswith(prefix)}
    frames = set(re.findall(r'@keyframes\s+([\w-]+)', css))
    fmap = {f: prefix + f for f in frames if not f.startswith(prefix)}
    def css_sub(m):
        n = m.group(1)
        return '.' + rename.get(n, n)
    new_css = re.sub(r'\.([a-zA-Z][\w-]*)(?![\w-])', css_sub, css)
    new_css = re.sub(r'@keyframes\s+([\w-]+)', lambda m: '@keyframes ' + fmap.get(m.group(1), m.group(1)), new_css)
    def anim_sub(m):
        decl = m.group(2)
        decl = re.sub(r'(?<![\w-])([a-zA-Z][\w-]*)(?![\w-])', lambda k: fmap.get(k.group(1), k.group(1)), decl)
        return m.group(1) + decl
    new_css = re.sub(r'(animation(?:-name)?\s*:)([^;}]*)', anim_sub, new_css)
    def cls_sub(m):
        toks = m.group(1).split(' ')
        out = []
        for t in toks:
            out.append(t if '{{' in t or '}}' in t else rename.get(t, t))
        return 'class="' + ' '.join(out) + '"'
    new_body = re.sub(r'class="([^"]*)"', cls_sub, body)
    new_head = re.sub(r'class="([^"]*)"', cls_sub, head)
    # Python-side generators sometimes put class names inside JS-built strings as well; leave script as is.
    return new_head + '<style>' + new_css + '</style>' + new_body + script, rename, dynamic

def check(html, prefix, dynamic):
    css = html.split('<style>', 1)[1].split('</style>', 1)[0]
    bad = []
    for sel in re.findall(r'([^{}]+)\{', css):
        if sel.strip().startswith('@') or re.match(r'^\s*(from|to|\d)', sel.strip()):
            continue
        for part in sel.split(','):
            comps = re.split(r'[\s>+~]+', part.strip())
            for comp in comps:
                cl = re.findall(r'\.([a-zA-Z][\w-]*)', comp)
                if cl and all(c in dynamic for c in cl) and not re.match(r'^[a-z]+', comp):
                    if not any(prefix in c for c in re.findall(r'\.([a-zA-Z][\w-]*)', part)):
                        bad.append(part.strip())
    return bad

if __name__ == '__main__':
    path, prefix = sys.argv[1], sys.argv[2]
    s = open(path).read()
    out, rename, dynamic = scope(s, prefix)
    open(path, 'w').write(out)
    print(path, 'renamed', len(rename), 'dynamic kept', sorted(dynamic), 'unsafe', check(out, prefix, dynamic))
