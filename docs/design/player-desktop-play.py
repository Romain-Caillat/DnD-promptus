"""Generate the player on a computer: Hugo plays Sef from his laptop (Joueur-Ordi.dc.html).

Six moments of the same evening as Jouer-Marc, laid out for a large screen: the scene, the hand of
cards, the die, the map, a combat turn, the loot. Same rules as the phone: one banner says what
is expected, one main button acts. Run from the repository root.
"""
import sys
sys.path.insert(0, 'docs/design')
from kit import imp, when, iff, head, note, top, laptop, side, component, board

P = 48
css = r'''
.me{display:flex;flex-direction:column;gap:10px;padding:0 14px 14px}
.me > .id{display:flex;align-items:flex-end;gap:12px}
.mates{display:flex;flex-direction:column;gap:6px;padding:0 14px 14px}
.mate{display:flex;align-items:center;gap:10px;font-size:13px;padding:6px 8px;border-radius:10px;background:#121212}
.mate > span:last-child{margin-left:auto;font-size:11px;color:#8C8C8C}
.jr{display:flex;flex-direction:column;gap:8px;padding:0 14px 14px}
.jl{font-size:12px;line-height:1.45;color:#A3A3A3;padding-left:10px;border-left:2px solid #2C2C2C}
.jl > b{color:#F2F2F2}
.jl.new{border-left-color:#F2F2F2;color:#D4D4D4}
.handrow{display:flex;gap:12px;align-items:flex-end;padding:10px 4px 0}
.cw{position:relative;transition:transform .2s}
.cw.sel{transform:translateY(-14px);filter:drop-shadow(0 0 14px rgba(255,255,255,.35))}
.cw > kbd{position:absolute;left:50%;bottom:-18px;transform:translateX(-50%);font-family:inherit;font-size:10px;padding:1px 5px;border-radius:4px;border:1px solid #3A3A3A;color:#6E6E6E}
.other{width:100px;height:140px;border-radius:10px;border:2px dashed #8C8C8C;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:6px;box-sizing:border-box}
.other > b{font-family:'Cinzel',serif;font-size:30px}
.mapw{position:relative;isolation:isolate;width:672px;height:432px;overflow:hidden;border-radius:12px;border:1px solid #2C2C2C;flex:none;align-self:center}
.reach{position:absolute;border-radius:6px;background:rgba(46,230,166,.28);box-shadow:inset 0 0 0 2px rgba(46,230,166,.8);z-index:940}
.reach.to{background:rgba(46,230,166,.6);box-shadow:inset 0 0 0 3px #F2F2F2}
.tgt{position:absolute;width:56px;height:56px;border-radius:50%;border:3px dashed #FF9F1C;z-index:960;box-sizing:border-box;animation:spin 5s linear infinite}
.init{display:flex;gap:8px;align-items:center}
.init > span{display:flex;align-items:center;gap:6px;padding:4px 10px 4px 4px;border-radius:10px;background:#141414;border:1px solid #2C2C2C;font-size:12px}
.init > span.now{border-color:#FFD60A;background:#1E1E1E}
.dz{display:flex;flex-direction:column;align-items:center;gap:10px;justify-content:center;flex:1}
.resn{display:flex;gap:10px;align-items:baseline;font-weight:700;font-size:34px}
.resn > .op{font-size:22px;color:#8C8C8C}
.win{font-family:'Cinzel',serif;font-weight:800;font-size:26px;color:#0A0A0A;background:#F2F2F2;padding:4px 18px;border-radius:8px;box-shadow:0 4px 0 #6E6E6E}
.dmg{position:absolute;z-index:980;font-weight:700;font-size:40px;color:#FF4D5E;-webkit-text-stroke:3px #000;paint-order:stroke}
'''


def plan(vue='joueur', combat='false', bx=4, by=6):
    return f'<div style="position: absolute; left: 0; top: -{P}px">' + imp('Plan', (672, 480), theme='crypte', carte='salle', pitch=P, vue=vue, grille='true', combat=combat, bx=bx, by=by) + '</div>'


CARDS = [dict(titre='Dague', type='Arme', texte='1d4 + 3, discrète.', valeur='+5', stat='atq', icon='epee', rarete='commune'),
         dict(titre='Se cacher', type='Action', texte='Discrétion contre leur Perception.', valeur='+7', stat='none', icon='oeil', rarete='aucune'),
         dict(titre='Crocheter', type='Dextérité', texte='Serrures et pièges.', valeur='+7', stat='none', icon='porte', rarete='commune'),
         dict(titre='Fouiller', type='Sagesse', texte='Chercher, observer.', valeur='+3', stat='none', icon='oeil', rarete='commune')]


def hand(sel):
    return ('<div class="handrow">' + ''.join(f'<div class="cw {"sel" if c["titre"] == sel else ""}">{imp("GameCard", (100, 140), w=100, **c)}<kbd>{i}</kbd></div>' for i, c in enumerate(CARDS, 1))
            + '<div class="cw"><div class="other"><b>…</b><span class="ttl" style="font-size: 12px">Autre</span></div><kbd>5</kbd></div></div>')


ART = '<div class="art" style="height: 230px"><span class="ttl" style="font-size: 22px">La salle de l’autel</span><span class="tag">[ILLUSTRATION]</span></div>'
C = []
C.append(when(1, ART + '<p class="nar" style="font-size: 24px">L’escalier s’arrête devant un autel de pierre noire. Des cendres encore tièdes. Quelqu’un est passé ici il y a moins d’une heure.</p>'
  + '<div class="secret" style="border-style: solid"><span class="lbl">Ce que Sef remarque</span><span>Des traces de pas fines, trop légères pour un homme.</span></div>'))
C.append(when(2, '<p class="nar" style="font-size: 22px">« Devant l’autel, que faites-vous ? »</p>' + hand('Crocheter')
  + '<div class="rl" style="margin-top: 18px"><span class="lbl">Crocheter</span><span>Le tiroir de l’autel est fermé. Dextérité +7, la difficulté est fixée par le MJ.</span></div>'
  + '<div class="ans"><b>Les autres</b><span>Borin fouille l’autel · Lyra surveille l’escalier</span></div>'))
C.append(when(3, '<div class="dz"><span class="ttl" style="font-size: 24px">Crocheter le tiroir</span><span style="font-size: 16px">Il te faut <b>14 ou plus</b></span>'
  + iff('rollIdle', '<button type="button" class="btn lt" style="height: auto; padding: 8px; background: none; outline: 0; box-shadow: none" onClick="{{roll}}">' + imp('De', (150, 150), de='d20', valeur=20, taille=150, anim='survol') + '</button><span class="mut">Clique sur le dé, ou appuie sur Espace</span>', True)
  + iff('rolled', imp('De', (150, 150), de='d20', valeur=11, taille=150, anim='fixe') + '<div class="resn"><span>11</span><span class="op">+ 7</span><span class="op">=</span><span>18</span></div><span class="win">Réussi !</span>') + '</div>'))
C.append(when(4, '<div class="mapw">' + plan('joueur', 'false', 4, 6)
  + ''.join(f'<span class="reach {"to" if (x, y) == (9, 7) else ""}" style="left: {x * P + 2}px; top: {y * P + 2}px; width: {P - 4}px; height: {P - 4}px"></span>' for x, y in [(8, 6), (8, 7), (9, 6), (9, 7), (10, 7), (7, 7), (7, 6), (10, 6)])
  + '</div><span class="mut">Les cases vertes : là où Sef peut aller ce tour-ci. Clic pour bouger, glisser pour voir plus loin.</span>'))
C.append(when(5, '<div class="init">' + ''.join(f'<span class="{"now" if n == "Sef" else ""}"><span class="pf" style="width: 28px; height: 36px">{imp("Sprite", (20, 26), perso=p, px=1, anim="repos")}</span>{n}</span>' for p, n in [('lyra', 'Lyra'), ('sef', 'Sef'), ('gobelin', 'Gobelin'), ('borin', 'Borin'), ('gobelin', 'Gobelin')]) + '</div>'
  + '<div class="mapw">' + plan('joueur', 'true') + f'<span class="tgt" style="left: {7 * P - 4}px; top: {7 * P - 4}px"></span>' + iff('hit', f'<span class="dmg" style="left: {7 * P + 6}px; top: {6 * P}px">−7</span>') + '</div>'
  + hand('Dague').replace('class="handrow"', 'class="handrow" style="padding-top: 0"')))
C.append(when(6, '<div class="row2" style="align-items: center; gap: 30px; justify-content: center; flex: 1">' + imp('GameCard', (220, 308), w=220, titre='Cape d’ombre', type='Objet', texte='Avantage pour se cacher dans le noir.', valeur='+2', stat='none', icon='oeil', rarete='rare')
  + '<div class="sec" style="max-width: 320px"><span class="lbl">Ton butin</span><span class="ttl" style="font-size: 26px">Cape d’ombre</span><span class="mut">Trouvée dans le tiroir de l’autel. Rare. Elle ira dans ton sac, ou sur tes épaules tout de suite.</span></div></div>'))

LEFT = ('<section class="panel"><div class="ph"><span class="lbl" style="color: #F2F2F2">Sef</span><span class="lbl">roublard · niv. 3</span></div><div class="me"><div class="id">'
        + imp('Perso', (80, 104), perso='sef', px=4, etat='aucun') + '<div class="sec" style="gap: 6px">' + imp('Coeurs', (151, 16), pv='{{hp}}', max=24, coeurs=6, px=2, anim='false') + '<span style="font-weight: 700; color: #FF4D5E">{{hp}} / 24 PV</span></div></div>'
        '<div class="gems">' + ''.join(f'<span class="gem">{imp("Gemme", (34, 34), stat=s, valeur=v, taille=34, anim="false")}{n}</span>' for s, v, n in [('ca', '15', 'Armure'), ('atq', '+5', 'Dague'), ('mvt', '6', 'Pas'), ('ini', '+4', 'Init.')]) + '</div></div></section>'
        '<section class="panel" style="flex: 1"><div class="ph"><span class="lbl" style="color: #F2F2F2">Le groupe</span></div><div class="mates">'
        + ''.join(f'<div class="mate"><span class="pf" style="width: 28px; height: 36px">{imp("Sprite", (20, 26), perso=p, px=1, anim="repos")}</span><b>{n}</b><span>{w}</span></div>' for p, n, w in [('borin', 'Borin', 'Marc · 15 / 31'), ('lyra', 'Lyra', 'Camille · 20 / 24')]) + '</div></section>')
RIGHT = ('<section class="panel" style="flex: 1"><div class="ph"><span class="lbl" style="color: #F2F2F2">Journal</span><span class="lbl">ce que le groupe sait</span></div><div class="jr">'
         '<div class="jl"><b>Le registre</b> : une page a été arrachée.</div><div class="jl"><b>La bague du gardien</b> : gravée d’un corbeau.</div>'
         '<div class="jl {{jNew}}">{{jLast}}</div></div></section>'
         '<section class="panel"><div class="ph"><span class="lbl" style="color: #F2F2F2">Musique</span></div><div class="co"><div class="yt"><span class="thumb">YOUTUBE</span><span class="eq"><i></i><i></i><i></i></span><span>Ombres de la crypte</span><span>son activé</span></div></div></section>')
TOP = top('Les Cendres de Valombre', ['Jeu', 'Carte', 'Sef', 'Journal'], 'Jeu', '<span>Session 3 · Hugo</span>')
lap = laptop(TOP, LEFT, ''.join(C), RIGHT, cols='280px 1fr 300px')

M = lambda lbl, sub, act, on='true': f'=[{lbl!r}, {sub!r}, {on}, () => {{ {act} }}]'
ROWS = {
    1: dict(bn=['you', 'Le MJ raconte · écoute'], main=M('Rien à faire pour l’instant', 'Le MJ lit la scène', '', 'false'),
            thought='Sur mon ordi, je vois tout d’un coup : la scène, ma fiche, le journal.',
            why='Sur un grand écran, le joueur garde sa fiche à gauche et le journal à droite. La scène reste au centre, avec le même rythme que sur téléphone.'),
    2: dict(bn=['you', 'À vous : que faites-vous ?'], main=M('Proposer au MJ', 'Crocheter le tiroir de l’autel', 'this.go(3)'),
            thought='Le tiroir. Si quelqu’un l’ouvre, c’est moi.',
            why='La main de cartes s’étale en bas, chaque carte a sa touche. « Autre… » laisse écrire une idée libre. Le MJ reçoit la proposition comme sur téléphone.'),
    3: dict(bn=['=s.rolled ? "you" : "turn"', '=s.rolled ? "Réussi : 18" : "À toi : lance le dé"'], main='=[s.rolled ? "Continuer" : "Lancer le dé", s.rolled ? "Le MJ révèle ce que tu trouves" : "Espace marche aussi", true, () => { if (s.rolled) this.go(4); else this.setState({ rolled: true }); }]',
            thought='=s.rolled ? "18. Le tiroir s’ouvre." : "Allez…"',
            why='Le dé se lance d’un clic ou d’une touche. Le serveur lance, le joueur voit le calcul complet : le dé, son bonus, le total.'),
    4: dict(bn=['you', 'Explore la salle'], main=M('Aller ici', 'Case surlignée · 2 cases', 'this.go(5)'),
            thought='Je me glisse vers l’eau, à l’ombre de la colonne.',
            why='Sur la carte, la souris remplace le doigt : les cases atteignables s’allument, un clic déplace. Le brouillard est le même que sur téléphone.'),
    5: dict(bn=['turn', '⚔ À toi de jouer'], main='=[s.hit ? "Finir mon tour" : "Frapper le gobelin", s.hit ? "Le gobelin a 0 PV" : "Dague · attaque sournoise", true, () => { if (s.hit) this.go(6); else this.setState({ hit: true }); }]',
            thought='=s.hit ? "Sournoise : 7 dégâts. Il tombe." : "Il est collé à Borin. Attaque sournoise."',
            why='En combat, l’ordre du tour est en haut, la carte au centre, les cartes en bas. Une carte, une cible, un bouton : rien à calculer.'),
    6: dict(bn=['you', '★ Tu as trouvé quelque chose'], main=M('Mettre la cape', 'Ou la ranger dans ton sac', 'this.end()'),
            thought='Une cape d’ombre. Pour un roublard. Romain me gâte.',
            why='Le butin tombe en grand au centre. Le joueur l’équipe d’un clic ; le MJ suit sans rien faire.'),
}
JS = component(6, ROWS, fresh='{ rolled: false, hit: false }', settled='{ rolled: m >= 3, hit: m >= 5 }',
               vals='''rollIdle: !s.rolled, rolled: s.rolled, roll: () => this.setState({ rolled: true }), hit: s.hit, hp: m >= 5 ? '17' : '24',
      jNew: m >= 3 ? 'new' : '', jLast: m >= 3 ? 'Le tiroir de l’autel : Sef l’a ouvert.' : 'Les cendres sont encore tièdes.',''',
               full=(2012, 1020), seul=(1440, 900))
parts = lap + iff('full', side('Hugo, le joueur', 'Joueur sur ordinateur', 6), True)
board('Joueur-Ordi', 'Jouer sur ordinateur', 'jo-', parts, JS, 6, (2012, 1020),
      {"x": 6840, "y": 15600, "w": 2012, "h": 1020, "title": "Jouer sur ordinateur · Sef", "is_interactive": True}, css)
