"""Generate the invitation storyboard (Invitation-MJ.dc.html): the seven moments of inviting a table.

Each frame is the Inviter-MJ board (GM laptop and Marc's phone) with seul=true, frozen on a moment once the players have acted.
Run from the repository root.
"""
import json, sys

FRAMES = [
    ('Le lien', 'Copie le lien et un message prêt à coller sur Discord.', 'Reçoit le message de Romain avec la carte d’invitation.', 'Le MJ invite là où sa table parle déjà ; pas de compte à créer.'),
    ('Ils arrivent', 'Voit sa table se remplir et Lyra arriver pour validation.', 'Choisit un pseudo et commence son personnage, sans compte.', 'L’appareil du joueur garde une clé secrète ; le serveur n’en garde que l’empreinte.'),
    ('Lyra', 'Relit la fiche de Camille, vérifiée par le co-MJ, et la valide.', 'Choisit sa classe : guerrier.', 'Le co-MJ vérifie les règles, le MJ lit l’histoire.'),
    ('Borin', 'Voit la Force à 18 signalée et le mot proposé pour Marc.', 'Attend la réponse du MJ.', 'Un point hors règles n’est pas un refus sec : c’est un mot, et le MJ peut accepter quand même.'),
    ('Le renvoi', 'Attend, puis voit ce qui a changé : Force 17, Sagesse 13.', 'Lit le mot, déplace le point, renvoie.', 'Le joueur corrige seul ; le MJ ne relit que la différence.'),
    ('La table complète', 'Ajoute en secret deux accroches tirées des histoires des joueurs.', 'Voit Borin validé et sa table.', 'Les histoires des joueurs nourrissent la campagne sans rien leur dévoiler.'),
    ('La date', 'Choisit le jeudi où tout le monde est libre et l’envoie.', 'Reçoit la date et l’ajoute à son agenda.', 'L’invitation se ferme sur un rendez-vous ; le salon ouvrira avant.'),
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
.fr{width:720px;height:345px;overflow:hidden;border-radius:10px;border:1px solid #2C2C2C;box-shadow:0 20px 40px rgba(0,0,0,.6)}
.cap{display:grid;grid-template-columns:130px 1fr;gap:6px 10px;font-size:13px;line-height:1.45;color:#D4D4D4;margin:0}
.cap > dt{font-size:10px;font-weight:700;letter-spacing:.16em;text-transform:uppercase;color:#8C8C8C;padding-top:3px}
.cap > dd{margin:0}
.rule{display:flex;gap:12px;align-items:flex-start;font-size:13px;color:#D4D4D4;line-height:1.45}
.rule > b{flex:none;width:24px;height:24px;border-radius:6px;background:#EDEDED;color:#0A0A0A;display:grid;place-items:center;font-family:'Cinzel',serif;box-shadow:0 2px 0 #6E6E6E}
'''

W, H = 2384, 1780
frames = ''.join(
    f'<div class="mo"><div class="mh"><b>{i}</b><span>{t}</span></div>'
    f'<div class="fr"><div style="width: 1880px; height: 900px; transform: scale(.383); transform-origin: 0 0"><dc-import name="Inviter-MJ" moment="{i}" seul="true" hint-size="1880px,900px"></dc-import></div></div>'
    f'<dl class="cap"><dt>Le MJ</dt><dd>{does}</dd><dt>Marc</dt><dd>{secret}</dd><dt>Pourquoi</dt><dd>{why}</dd></dl></div>'
    for i, (t, does, secret, why) in enumerate(FRAMES, 1))

RULES = [
    ('Un lien, pas de compte.', 'Le joueur choisit un pseudo ; son appareil garde sa place.'),
    ('Le MJ valide chaque personnage.', 'Le co-MJ vérifie les règles et écrit le mot ; le MJ tranche.'),
    ('Les histoires servent la campagne.', 'Ce que les joueurs écrivent devient des accroches secrètes dans le graphe.'),
]
html = f'''<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<title>Inviter la table</title>
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
<h1 class="ttl" style="margin: 0; font-size: 50px">Inviter la table</h1>
<p style="margin: 0; font-size: 16px; color: #A3A3A3; line-height: 1.5">Romain invite Camille, Marc et Hugo dans « Les Cendres de Valombre ». À gauche son ordinateur, à droite le téléphone de Marc. Chaque personnage passe par le MJ avant d’entrer en jeu. La planche « Inviter » la rejoue en entier.</p>
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
out = 'docs/design/canvas/Invitation-MJ.dc.html'
open(out, 'w').write(html)
d = json.load(open('docs/design/canvas/canvas.json'))
d['boards'].setdefault('Invitation-MJ.dc.html', {"x": 2560, "y": 11400, "w": W, "h": H, "title": "MJ · inviter la table"})  # keep a layout set on the canvas
if 'Invitation-MJ.dc.html' not in d['order']: d['order'].append('Invitation-MJ.dc.html')
json.dump(d, open('docs/design/canvas/canvas.json', 'w'), ensure_ascii=False, indent=2)

sys.path.insert(0, 'docs/design')
from scope import scope, check
_out, _ren, _dyn = scope(open(out).read(), 'imj-', keep=set())
open(out, 'w').write(_out)
print(len(_out), 'unsafe:', check(_out, 'imj-', _dyn))
