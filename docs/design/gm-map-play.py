"""Generate the map editor: Romain retouches the altar room the co-GM proposed (Carte-MJ.dc.html).

Seven moments on the GM laptop: the proposal drawn from the story graph, walls and terrain,
doors and decor, hidden objects, lights and mood, the players' view with fog, links between
scales. The map is the Plan component; the editing is drawn over it. Run from the repository root.
"""
import sys
sys.path.insert(0, 'docs/design')
from kit import imp, when, iff, head, note, chk, top, laptop, side, component, board

P = 48
css = r'''
.mapw{position:relative;isolation:isolate;width:672px;height:432px;overflow:hidden;border-radius:12px;border:1px solid #2C2C2C;flex:none;align-self:center}
.ov{position:absolute;inset:0;z-index:950;pointer-events:none}
.sel{position:absolute;border:2px dashed #F2F2F2;border-radius:4px;background:rgba(242,242,242,.08);box-sizing:border-box}
.sel.warm{border-color:#FFD60A;background:rgba(255,214,10,.1)}
.mk{position:absolute;transform:translate(-50%,-100%);font-size:10px;font-weight:700;letter-spacing:.06em;padding:3px 7px;border-radius:5px;background:#0A0A0A;border:1px solid #F2F2F2;white-space:nowrap}
.mk.hid{border-style:dashed;color:#FFD60A;border-color:#FFD60A}
.mk.ex{background:#EDEDED;color:#0A0A0A;border-color:#0A0A0A}
.light{position:absolute;border-radius:50%;transform:translate(-50%,-50%);background:radial-gradient(circle,rgba(255,190,90,.35),rgba(255,190,90,.08) 55%,transparent 70%)}
.dark{position:absolute;inset:0;background:rgba(0,0,0,.45);-webkit-mask:radial-gradient(circle at 120px 140px,transparent 70px,#000 120px),radial-gradient(circle at 400px 330px,transparent 70px,#000 120px);mask:radial-gradient(circle at 120px 140px,transparent 70px,#000 120px);mask-composite:intersect}
.brush{position:absolute;width:40px;height:40px;border-radius:50%;border:2px solid #F2F2F2;transform:translate(-50%,-50%);box-shadow:0 0 0 2px #0A0A0A}
.tools{display:flex;flex-direction:column;gap:3px;padding:0 10px 10px}
.tl{display:flex;align-items:center;gap:10px;height:36px;padding:0 12px;border-radius:10px;font-size:13px;color:#8C8C8C}
.tl > kbd{margin-left:auto;font-family:inherit;font-size:10px;padding:1px 5px;border-radius:4px;border:1px solid #3A3A3A;color:#6E6E6E}
.tl.cur{background:#EDEDED;color:#0A0A0A;font-weight:700;box-shadow:0 3px 0 #8A8A8A}
.tl.cur > kbd{border-color:#5A5A5A;color:#5A5A5A}
.pal{display:flex;gap:8px;flex-wrap:wrap}
.pal > span{display:inline-flex;align-items:center;gap:6px;padding:6px 10px;border-radius:8px;background:#141414;border:1.5px solid #2C2C2C;font-size:12px;color:#D4D4D4}
.pal > span.on{background:#EDEDED;color:#0A0A0A;border-color:#0A0A0A}
.pal > span > i{width:14px;height:14px;border-radius:3px;display:inline-block}
.crumb{display:flex;gap:8px;align-items:center;font-size:12px;color:#8C8C8C}
.crumb > b{color:#F2F2F2}
.insp{display:flex;flex-direction:column;gap:6px;padding:12px 14px;border-radius:12px;background:#0D0D0D;border:1px solid #333;font-size:13px;line-height:1.45;color:#D4D4D4}
.insp > b{font-family:'Cinzel',serif;font-size:16px;color:#F2F2F2}
.vw{display:flex;gap:4px;padding:3px;border-radius:10px;background:#0D0D0D;border:1px solid #2C2C2C;align-self:flex-start}
.vw > span{font-size:12px;font-weight:700;padding:6px 12px;border-radius:7px;color:#8C8C8C}
.vw > span.on{background:#EDEDED;color:#0A0A0A}
'''

c = lambda x, y: (x * P, y * P)


def plan(vue='mj'):
    return f'<div style="position: absolute; left: 0; top: -{P}px">' + imp('Plan', (672, 480), theme='crypte', carte='salle', pitch=P, vue=vue, grille='true', bx=4, by=6) + '</div>'


def mk(x, y, text, cls=''):
    px, py = x * P + P // 2, y * P + 4
    return f'<span class="mk {cls}" style="left: {px}px; top: {py}px">{text}</span>'


def sel(x0, y0, x1, y1, cls=''):
    return f'<span class="sel {cls}" style="left: {x0 * P}px; top: {y0 * P}px; width: {(x1 - x0 + 1) * P}px; height: {(y1 - y0 + 1) * P}px"></span>'


def mapw(vue, overlay):
    return f'<div class="mapw">{plan(vue)}<div class="ov">{overlay}</div></div>'


TOOLS = [('Sélection', 'V'), ('Murs', 'M'), ('Sol et eau', 'S'), ('Portes', 'P'), ('Décors', 'D'), ('Objets cachés', 'O'), ('Lumières', 'L'), ('Brouillard', 'B'), ('Sorties', 'X')]
CUR = {1: 'Sélection', 2: 'Murs', 3: 'Décors', 4: 'Objets cachés', 5: 'Lumières', 6: 'Brouillard', 7: 'Sorties'}

C = []
C.append(when(1, head('La salle de l’autel', 'proposée par le co-MJ') + mapw('mj', mk(3, 2, 'Autel') + mk(12, 1, 'Coffre · caché', 'hid') + mk(5, 6, '2 gobelins') + mk(3, 5, 'Vers l’escalier', 'ex'))
  + '<div class="crumb"><span>Monde</span>›<span>Valombre</span>›<span>La crypte</span>›<b>La salle de l’autel</b></div>'))
C.append(when(2, head('Murs et terrains', 'peindre au pinceau') + mapw('mj', sel(8, 6, 12, 7, 'warm') + '<span class="brush" style="left: 500px; top: 330px"></span>' + mk(10, 6, 'Eau peu profonde · terrain difficile'))
  + '<div class="pal"><span><i style="background: #3A3A40"></i>Mur</span><span><i style="background: #2A2622"></i>Dalles</span><span class="on"><i style="background: #1E2C3A"></i>Eau peu profonde</span><span><i style="background: #2E2A20"></i>Gravats</span><span><i style="background: #050505"></i>Gouffre</span></div>'))
C.append(when(3, head('Portes et décors', 'poser, tourner, verrouiller') + mapw('mj', sel(7, 3, 7, 3) + mk(7, 3, 'Porte secrète') + mk(3, 2, 'Autel') + mk(1, 1, 'Sarcophage') + mk(5, 1, 'Sarcophage') + mk(10, 7, 'Colonne brisée'))
  + '<div class="pal"><span class="on">Autel</span><span>Sarcophage</span><span>Colonne</span><span>Brasero</span><span>Tas d’os</span><span>Statue</span><span>+ 14 du pack crypte</span></div>'))
C.append(when(4, head('Objets cachés', 'ce qu’un jet peut révéler') + mapw('mj', sel(3, 2, 3, 2, 'warm') + mk(3, 2, 'La page arrachée · SAG 13', 'hid') + mk(12, 1, 'Coffre · caché', 'hid') + mk(7, 3, 'Porte secrète · SAG 15', 'hid'))
  + '<span class="mut">Un objet caché n’existe pas pour les joueurs tant qu’un jet ou ton geste ne l’a pas révélé. Le serveur garde le secret, pas leur téléphone.</span>'))
C.append(when(5, head('Lumières et ambiance', 'ce que les personnages voient') + mapw('mj', '<span class="light" style="left: 168px; top: 120px; width: 260px; height: 260px"></span><span class="light" style="left: 408px; top: 312px; width: 240px; height: 240px"></span>' + mk(3, 2, 'Brasero · 4 cases') + mk(8, 6, 'Torche murale · 3 cases'))
  + '<div class="rl"><span class="lbl">Ambiance</span><span>Sombre, cendres dans l’air. Musique : « Ombres de la crypte ». Borin voit dans le noir, Lyra et Sef ont besoin des braseros.</span></div>'))
C.append(when(6, head('Ce que voient les joueurs', 'aperçu') + '<div class="vw"><span>Toi</span><span class="on">Les joueurs</span><span>La TV</span></div>' + mapw('joueur', sel(8, 0, 13, 4) + mk(10, 2, 'Brouillard · levé quand la porte s’ouvre'))
  + '<span class="mut">Le brouillard couvre la salle est. Le coffre et la porte secrète n’apparaissent pas : ils ne sont pas cachés sous le brouillard, ils ne sont pas envoyés.</span>'))
C.append(when(7, head('Relier les échelles', 'd’où on vient, où on va') + mapw('mj', mk(3, 5, 'Escalier → l’entrée de la crypte', 'ex') + mk(12, 6, 'Puits → crypte, niveau 2', 'ex') + mk(7, 3, 'Porte nord → session 4', 'ex'))
  + '<div class="crumb"><span>Monde · hexagone 12-3</span>›<span>Valombre · la crypte</span>›<b>La salle de l’autel</b>›<span>Rencontre : 2 gobelins</span></div>'))

LEFT = ('<section class="panel" style="flex: 1"><div class="ph"><span class="lbl" style="color: #F2F2F2">Outils</span></div><div class="tools">'
        + ''.join(f'<span class="tl {{{{t{i}}}}}">{n}<kbd>{k}</kbd></span>' for i, (n, k) in enumerate(TOOLS)) + '</div></section>')
INSP = {
    1: ('Tirée du graphe', 'Le nœud « La salle de l’autel » demande : un autel, la page cachée, deux gobelins, une sortie vers l’escalier. Décor : crypte.'),
    2: ('Eau peu profonde', 'Terrain difficile : chaque case compte double pour le mouvement. Le serveur l’applique, les joueurs le voient en jeu.'),
    3: ('Porte secrète', 'Fermée, invisible tant qu’un jet de Sagesse 15 ne la trouve pas. Mène à la salle est.'),
    4: ('La page arrachée', 'Indice du nœud. Sagesse (Perception) DD 13. Révélée, elle part en carte dans le journal de tous.'),
    5: ('Brasero', 'Lumière vive sur 4 cases, faible sur 4 de plus. Allumé au début de la scène.'),
    6: ('Brouillard', 'Zone est : levée quand la porte secrète s’ouvre. Tu peux aussi la lever d’un geste en jeu.'),
    7: ('Sorties', 'Trois sorties. Chacune ouvre la carte suivante au bon moment, sans que tu aies à la chercher.'),
}
RIGHT = (''.join(when(n, f'<section class="panel"><div class="ph"><span class="lbl" style="color: #F2F2F2">Sélection</span></div><div class="co"><div class="insp"><b>{t}</b><span>{d}</span></div></div></section>') for n, (t, d) in INSP.items())
         + '<section class="panel"><div class="ph"><span class="lbl" style="color: #F2F2F2">Le co-MJ</span></div><div class="co"><div class="note"><span class="lbl">Propose</span>{{coTxt}}</div></div></section>')
TOP = top('Les Cendres de Valombre', ['Histoire', 'Règles', 'Cartes', 'Médias', 'Joueurs', 'Sessions'], 'Cartes', '<span>{{saveTxt}}</span>')
lap = laptop(TOP, LEFT, ''.join(C), RIGHT, cols='220px 1fr 340px')

M = lambda lbl, sub, act, on='true': f'=[{lbl!r}, {sub!r}, {on}, () => {{ {act} }}]'
ROWS = {
    1: dict(bn=['you', 'Le co-MJ a dessiné la salle de l’autel'], main=M('Retoucher', 'L’éditeur, outil par outil', 'this.go(2)'),
            thought='Pas mal. Mais je veux de l’eau, une porte secrète et des sarcophages.', coTxt='Je l’ai tirée du graphe : tout ce que la scène demande y est déjà.',
            why='Chaque lieu du graphe reçoit une carte proposée par l’IA, dans le décor choisi. Le MJ part de là au lieu d’une page blanche.'),
    2: dict(bn=['you', 'Il peint l’eau'], main=M('Poser les décors', 'Portes, sarcophages', 'this.go(3)'),
            thought='Une flaque au sud. Les gobelins vont patauger.', coTxt='L’eau ralentit tout le monde. Les gobelins y sont à désavantage pour esquiver.',
            why='Murs et terrains se peignent au pinceau, sur la grille. Un terrain a une règle (difficile, gouffre) que le serveur applique en jeu.'),
    3: dict(bn=['you', 'Il pose une porte secrète'], main=M('Cacher la page', 'Sous l’autel', 'this.go(4)'),
            thought='Une porte secrète vers la salle est. Lyra va la trouver, j’espère.', coTxt='La porte secrète relie la salle est au front du culte. Je l’ajoute au graphe ?',
            why='Les décors viennent du pack de l’univers. Une porte est un objet posé sur la carte, qui peut être fermée, verrouillée ou secrète.'),
    4: dict(bn=['you', 'Les objets cachés'], main=M('Régler la lumière', 'Braseros et torches', 'this.go(5)'),
            thought='La page sous l’autel, Sagesse 13. Comme dans ma prépa.', coTxt='La page est l’indice 3 sur 3 de la révélation « la porte nord ».',
            why='Ce qui est caché est un objet posé par-dessus la carte, avec le jet qui le révèle. Les joueurs ne le reçoivent qu’une fois trouvé.'),
    5: dict(bn=['you', 'Lumières et ambiance'], main=M('Voir comme les joueurs', 'Avec le brouillard', 'this.go(6)'),
            thought='Deux braseros. Le reste dans le noir, sauf pour Borin.', coTxt='Borin voit dans le noir : il verra le gobelin qui guette avant les autres.',
            why='La lumière décide de ce que chaque personnage voit, selon ses sens. L’ambiance relie la carte à la musique de la scène.'),
    6: dict(bn=['you', 'L’aperçu des joueurs'], main=M('Relier les sorties', 'Escalier, puits, porte nord', 'this.go(7)'),
            thought='Rien ne dépasse. La salle est reste un mystère.', coTxt='Aucun objet caché n’est envoyé aux joueurs. Vérifié.',
            why='Le MJ voit sa carte comme la verront les joueurs et la TV. Ce qui est caché n’est pas recouvert : il n’est pas envoyé du tout.'),
    7: dict(bn=['=s.ok ? "you" : "you"', '=s.ok ? "Carte validée" : "Trois sorties reliées"'], main=M('Valider la carte', 'Elle rejoint la campagne', 'this.setState({ ok: true })', '!s.ok'),
            thought='=s.ok ? "Validée. Plus que la salle est." : "Le puits mène au niveau 2. Ils ne sont pas prêts."', coTxt='Il reste la salle est à dessiner. Je te la propose ?',
            why='Les échelles se relient : le monde en hexagones mène au lieu, le lieu à la rencontre. En jeu, chaque sortie ouvre la bonne carte.'),
}
JS = component(7, ROWS, fresh='{ ok: false }', settled='{ ok: m >= 7 }',
               vals="...Object.fromEntries(" + str([n for n, _ in TOOLS]) + ".map((n, i) => ['t' + i, n === " + str(CUR) + "[m] ? 'cur' : ''])), saveTxt: s.ok ? 'Carte validée' : 'Brouillon · enregistré',",
               full=(2012, 1020), seul=(1440, 900))
parts = lap + iff('full', side('Romain, le MJ', 'L’éditeur de carte', 7), True)
board('Carte-MJ', 'L’éditeur de carte', 'em-', parts, JS, 7, (2012, 1020),
      {"x": 4720, "y": 15600, "w": 2012, "h": 1020, "title": "Cartes · l’éditeur de la salle de l’autel", "is_interactive": True}, css)
