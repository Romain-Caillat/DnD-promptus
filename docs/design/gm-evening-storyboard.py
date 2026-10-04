"""Generate the GM storyboard (Soiree-MJ.dc.html): the twelve moments of Marc's evening on the GM laptop.

Each frame is the Mener-MJ board with seul=true, frozen on a moment once the players have acted.
Run from the repository root.
"""
import json, sys

FRAMES = [
    ('Le salon', 'Voit qui est arrivé et si le son marche chez chacun, puis lance la session.', 'Rien encore : la session n’a pas commencé.', 'Le MJ ne lance que quand tout le monde est prêt, sans demander à voix haute.'),
    ('Précédemment…', 'Laisse passer le résumé écrit par le co-MJ, relu avant la partie.', 'Les PV de Borin, le chef gobelin qui attend derrière la porte est.', 'La partie repart sans que le MJ ait à se souvenir de tout.'),
    ('La scène', 'Lit le texte à voix haute pendant qu’il s’affiche partout.', 'Où est la page arrachée et quel jet la trouve, les gobelins derrière la porte.', 'Le texte public et les notes secrètes sont côte à côte, jamais mélangés.'),
    ('Que faites-vous ?', 'Reçoit la proposition de Marc en carte et demande un jet.', 'Rien de plus : la décision est la sienne.', 'Une demande = une carte, avec ses réponses possibles. Le MJ tranche en un clic.'),
    ('Le jet de Borin', 'Regarde le dé tomber et ce qu’il avait prévu pour la réussite et l’échec.', 'Le résultat raté qu’il n’utilisera pas.', 'Le serveur lance ; le MJ n’a rien à calculer.'),
    ('L’indice', 'Révèle la page et voit où en est la révélation : 3 indices sur 3.', 'Ce que les indices permettent de comprendre.', 'Le MJ sait si les joueurs ont de quoi avancer, sans leur souffler la réponse.'),
    ('La carte', 'Voit toute la crypte et ce que les joueurs ne voient pas, puis lance la rencontre.', 'La zone en brouillard, le gobelin qui guette, le coffre.', 'Un seul plan pour tout le monde ; le brouillard ne s’applique qu’aux joueurs.'),
    ('Le combat', 'Joue le premier gobelin sur proposition du co-MJ.', 'Les PV des gobelins, dans l’ordre du tour.', 'Le MJ joue les adversaires en validant, pas en remplissant des fiches.'),
    ('Le tour de Borin', 'Regarde le serveur résoudre la frappe de Marc.', 'Il reste 7 PV au gobelin.', 'Le tour d’un joueur ne demande rien au MJ, sauf si le joueur propose une idée.'),
    ('La riposte', 'Joue la riposte, puis voit le butin prévu pour ce combat.', 'La hache légendaire qui attend la fin du combat.', 'La récompense est préparée ; le MJ la valide au bon moment.'),
    ('Le butin', 'Voit Marc équiper la hache et l’expérience de chacun.', 'Rien : tout est public maintenant.', 'Le MJ suit la récompense sans la manipuler.'),
    ('Fin de soirée', 'Relit les deux récapitulatifs, le sien et celui des joueurs, puis publie.', 'Le récap MJ : fils ouverts, menace à faire avancer.', 'Le MJ ferme la soirée en deux minutes, et jeudi est déjà prêt.'),
]

css = r'''
body{margin:0;background:#0A0A0A;overflow:hidden}
.pm{font-family:'Chakra Petch',system-ui,sans-serif;color:#F2F2F2;background:radial-gradient(120% 40% at 50% 0%,#1C1C1C 0,#0C0C0C 55%,#050505 100%);-webkit-font-smoothing:antialiased}
.ttl{font-family:'Cinzel',Georgia,serif;font-weight:800}
.lbl{font-size:11px;font-weight:600;letter-spacing:.2em;text-transform:uppercase;color:#8C8C8C}
.grid{display:grid;grid-template-columns:repeat(3,720px);gap:56px 48px}
.mo{display:flex;flex-direction:column;gap:14px}
.mh{display:flex;align-items:baseline;gap:10px}
.mh > b{font-family:'Cinzel',serif;font-size:13px;width:28px;height:28px;border-radius:7px;display:inline-grid;place-items:center;background:#EDEDED;color:#0A0A0A;box-shadow:0 2px 0 #6E6E6E;flex:none}
.mh > span{font-family:'Cinzel',serif;font-weight:800;font-size:20px}
.fr{width:720px;height:450px;overflow:hidden;border-radius:10px;border:1px solid #2C2C2C;box-shadow:0 20px 40px rgba(0,0,0,.6)}
.cap{display:grid;grid-template-columns:130px 1fr;gap:6px 10px;font-size:13px;line-height:1.45;color:#D4D4D4;margin:0}
.cap > dt{font-size:10px;font-weight:700;letter-spacing:.16em;text-transform:uppercase;color:#8C8C8C;padding-top:3px}
.cap > dd{margin:0}
.rule{display:flex;gap:12px;align-items:flex-start;font-size:13px;color:#D4D4D4;line-height:1.45}
.rule > b{flex:none;width:24px;height:24px;border-radius:6px;background:#EDEDED;color:#0A0A0A;display:grid;place-items:center;font-family:'Cinzel',serif;box-shadow:0 2px 0 #6E6E6E}
'''

W, H = 2384, 2760
frames = ''.join(
    f'<div class="mo"><div class="mh"><b>{i}</b><span>{t}</span></div>'
    f'<div class="fr"><div style="width: 1440px; height: 900px; transform: scale(.5); transform-origin: 0 0"><dc-import name="Mener-MJ" moment="{i}" seul="true" hint-size="1440px,900px"></dc-import></div></div>'
    f'<dl class="cap"><dt>Le MJ</dt><dd>{does}</dd><dt>Lui seul voit</dt><dd>{secret}</dd><dt>Pourquoi</dt><dd>{why}</dd></dl></div>'
    for i, (t, does, secret, why) in enumerate(FRAMES, 1))

RULES = [
    ('Une action principale par moment.', 'Une bannière dit au MJ ce qui l’attend, un seul gros bouton fait avancer la soirée.'),
    ('Le public et le secret côte à côte.', 'Ce qui part aux joueurs est en clair, ce qui reste au MJ est en pointillés.'),
    ('L’IA propose, le MJ valide.', 'Le co-MJ écrit des brouillons et joue des suggestions ; rien ne part sans un clic.'),
]
html = f'''<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<title>Soirée côté MJ</title>
<script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Cinzel:wght@600;700;800&amp;family=Chakra+Petch:wght@400;500;600;700&amp;family=Cormorant+Garamond:ital,wght@1,500;1,600&amp;display=swap">
<style>{css}</style>
</helmet>
<div class="pm" style="width: {W}px; height: {H}px; position: relative; overflow: hidden">
<div style="padding: 48px 64px 32px; display: flex; gap: 80px; align-items: flex-end">
<div style="display: flex; flex-direction: column; gap: 10px; max-width: 1000px">
<div class="lbl">MJ · ordinateur</div>
<h1 class="ttl" style="margin: 0; font-size: 50px">La soirée côté MJ</h1>
<p style="margin: 0; font-size: 16px; color: #A3A3A3; line-height: 1.5">La même soirée, vue par Romain depuis son ordinateur. Il mène tout d’un seul écran : la soirée à gauche, le moment au centre, le co-MJ et le journal à droite. Chaque écran est figé une fois que les joueurs ont agi ; la planche « Mener » la rejoue en entier.</p>
</div>
<div style="display: flex; flex-direction: column; gap: 10px; max-width: 900px">
''' + ''.join(f'<div class="rule"><b>{n}</b><span><strong style="color: #F2F2F2">{a}</strong> {b}</span></div>' for n, (a, b) in enumerate(RULES, 1)) + f'''
</div>
</div>
<div class="grid" style="padding: 0 64px">{frames}</div>
</div>
</x-dc>
<script type="text/x-dc" data-dc-script data-props='{{"$preview":{{"width":{W},"height":{H}}}}}'>
class Component extends DCLogic {{
  renderVals() {{
    return {{}};
  }}
}}
</script>
</body>
</html>
'''
out = 'docs/design/canvas/Soiree-MJ.dc.html'
open(out, 'w').write(html)
d = json.load(open('docs/design/canvas/canvas.json'))
d['boards'].setdefault('Soiree-MJ.dc.html', {"x": 4080, "y": 7200, "w": W, "h": H, "title": "MJ · la soirée côté MJ"})  # keep a layout set on the canvas
if 'Soiree-MJ.dc.html' not in d['order']: d['order'].append('Soiree-MJ.dc.html')
json.dump(d, open('docs/design/canvas/canvas.json', 'w'), ensure_ascii=False, indent=2)

sys.path.insert(0, 'docs/design')
from scope import scope, check
_out, _ren, _dyn = scope(open(out).read(), 'smj-', keep=set())
open(out, 'w').write(_out)
print(len(_out), 'unsafe:', check(_out, 'smj-', _dyn))
