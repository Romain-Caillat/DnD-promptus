"""Generate the rule system editor: Romain adjusts D&D 5e (SRD) for his campaign (Regles-MJ.dc.html).

Seven moments on the GM laptop: the preset, the stats, how a check resolves, the actions the
players will see as cards, a house rule written in French and formalised by the co-GM, character
creation limits, and a simulated fight before locking the version. Run from the repository root.
"""
import sys
sys.path.insert(0, 'docs/design')
from kit import imp, when, iff, head, note, chk, top, laptop, side, component, board

css = r'''
.nav2{display:flex;flex-direction:column;gap:4px;padding:0 10px 10px}
.nav2 > span{display:flex;align-items:center;gap:10px;height:40px;padding:0 12px;border-radius:10px;font-size:13px;color:#8C8C8C}
.nav2 > span > small{margin-left:auto;font-size:11px}
.nav2 > span.cur{background:#EDEDED;color:#0A0A0A;font-weight:700;box-shadow:0 3px 0 #8A8A8A}
.nav2 > span.done{color:#D4D4D4}
.tbl{display:flex;flex-direction:column;gap:4px}
.tr{display:grid;grid-template-columns:70px 1fr 1.3fr 90px;gap:12px;align-items:center;padding:8px 12px;border-radius:10px;background:#121212;font-size:13px;color:#D4D4D4}
.tr.th{background:none;padding:0 12px}
.tr > b{font-size:12px;letter-spacing:.12em}
.tr.ed{border:1.5px solid #F2F2F2;background:#1C1C1C}
.tr > code{font-family:'Chakra Petch',monospace;font-size:12px;color:#A3A3A3}
.dd{display:grid;grid-template-columns:repeat(4,1fr);gap:10px}
.dc{display:flex;flex-direction:column;align-items:center;gap:4px;padding:12px;border-radius:12px;background:#141414;border:1px solid #2C2C2C;font-size:12px;color:#A3A3A3}
.dc > b{font-size:26px;color:#F2F2F2}
.res{display:flex;align-items:center;gap:20px;padding:14px 16px;border-radius:12px;background:#0D0D0D;border:1px solid #333}
.res > div{display:flex;flex-direction:column;gap:4px;font-size:13px;line-height:1.5;color:#D4D4D4}
.acts{display:grid;grid-template-columns:repeat(3,1fr);gap:10px}
.act{display:flex;gap:10px;align-items:flex-start;padding:10px;border-radius:12px;background:#141414;border:1px solid #2C2C2C;font-size:12px;line-height:1.4;color:#A3A3A3}
.act > div{display:flex;flex-direction:column;gap:3px}
.act > div > b{font-size:13px;color:#F2F2F2}
.act.off{opacity:.4}
.tog{width:30px;height:18px;border-radius:9px;background:#EDEDED;position:relative;flex:none;margin-left:auto}
.tog::after{content:'';position:absolute;right:2px;top:2px;width:14px;height:14px;border-radius:50%;background:#0A0A0A}
.act.off .tog{background:#2C2C2C}
.act.off .tog::after{left:2px;right:auto;background:#5A5A5A}
.formal{display:grid;grid-template-columns:110px 1fr;gap:6px 12px;padding:14px 16px;border-radius:12px;background:#0D0D0D;border:1px solid #333;font-size:13px;line-height:1.45;color:#D4D4D4}
.formal > span:nth-child(odd){font-size:10px;font-weight:700;letter-spacing:.16em;text-transform:uppercase;color:#8C8C8C;padding-top:3px}
.formal.on{border:1.5px solid #F2F2F2}
.case{display:flex;align-items:center;gap:10px;font-size:13px;padding:8px 12px;border-radius:10px;background:#121212;color:#D4D4D4}
.case > i{font-style:normal;font-size:11px;font-weight:700;padding:2px 6px;border-radius:5px;background:#EDEDED;color:#0A0A0A;flex:none}
.case > i.no{background:#2C2C2C;color:#A3A3A3}
.lim{display:grid;grid-template-columns:1fr 1fr;gap:10px}
.sim{display:grid;grid-template-columns:repeat(3,1fr);gap:10px}
.big{display:flex;flex-direction:column;gap:4px;padding:14px;border-radius:12px;background:#141414;border:1px solid #2C2C2C;font-size:12px;color:#A3A3A3}
.big > b{font-size:30px;color:#F2F2F2}
.bars{display:flex;align-items:flex-end;gap:4px;height:90px;padding:10px 12px;border-radius:12px;background:#0D0D0D;border:1px solid #333}
.bars > i{flex:1;background:#5A5A5A;border-radius:3px 3px 0 0}
.bars > i.hi{background:#F2F2F2}
.ver{display:flex;align-items:center;gap:10px;padding:10px 12px;border-radius:10px;background:#121212;font-size:13px}
'''

TABS = ['Préréglage', 'Statistiques', 'Les jets', 'Actions', 'Règles maison', 'Création', 'Tester']


def stage(rows):
    return ('<div class="tbl"><div class="tr th"><span class="lbl">Clé</span><span class="lbl">Nom affiché</span><span class="lbl">Formule</span><span class="lbl">Gemme</span></div>'
            + ''.join(f'<div class="tr {cls}"><b>{k}</b><span>{n}</span><code>{f}</code><span>{g}</span></div>' for k, n, f, g, cls in rows) + '</div>')


GEM = lambda s, v: imp('Gemme', (30, 30), stat=s, valeur=v, taille=30, anim='false')
C = []
C.append(when(1, head('Le système de règles', 'D&D 5e · SRD 5.1')
  + '<div class="rl"><span class="lbl">Préréglage</span><span><b>D&amp;D 5e (SRD)</b>, la version libre de droits. Les autres : Simple d20, Horreur (d6), Science-fiction (2d6).</span></div>'
  + '<div class="rl"><span class="lbl">Contient</span><span>6 caractéristiques, 5 statistiques de jeu, jets d20 contre une difficulté, 18 actions, 4 classes, 9 états, les règles de voyage et de repos.</span></div>'
  + '<div class="rl"><span class="lbl">Utilisé par</span><span>Les Cendres de Valombre. Une copie est faite pour cette campagne : tes changements ne touchent pas le préréglage.</span></div>'
  + '<span class="mut">Tout est lisible par toi et par le serveur, qui juge chaque règle en jeu. Les joueurs n’en voient que ce qui les concerne, sous forme de cartes.</span>'))
C.append(when(2, head('Les statistiques', 'ce que les gemmes affichent') + stage([
    ('PV', 'Vie', 'dé de vie + CON par niveau', GEM('pv', '13'), ''), ('CA', 'Armure', '10 + DEX ou armure', GEM('ca', '18'), ''),
    ('ATQ', 'Attaque', 'maîtrise + FOR ou DEX', GEM('atq', '+5'), ''), ('MVT', '{{mvtName}}', '{{mvtFormula}}', GEM('mvt', '{{mvtVal}}'), 'ed'),
    ('INI', 'Initiative', 'DEX', GEM('ini', '+0'), '')])
  + '<span class="mut">Romain compte le mouvement en cases plutôt qu’en mètres : le serveur convertit, les joueurs voient « 5 cases ».</span>'))
C.append(when(3, head('Les jets', 'comment un test se résout')
  + '<div class="res">' + imp('De', (90, 90), de='d20', valeur=14, taille=90, anim='fixe') + '<div><b style="font-size: 16px; color: #F2F2F2">d20 + caractéristique + maîtrise ≥ difficulté</b><span>Avantage : deux dés, le meilleur. Désavantage : le pire. 20 naturel : critique en attaque. 1 naturel : échec.</span></div></div>'
  + '<span class="lbl">Les difficultés que tu donnes</span><div class="dd">' + ''.join(f'<div class="dc"><b>{v}</b>{n}</div>' for v, n in [('{{dd1}}', 'Facile'), ('13', 'Moyen'), ('16', 'Difficile'), ('20', 'Héroïque')]) + '</div>'
  + '<div class="rl"><span class="lbl">Changé</span><span>« Facile » passe de 10 à {{dd1}} : à 10, presque tout réussissait au niveau 3.</span></div>'))
C.append(when(4, head('Les actions', 'les cartes que les joueurs auront') + '<div class="acts">'
  + ''.join(f'<div class="act {o}"><span>{imp("Objet", (36, 36), objet=ob, rarete="commune", taille=36, pips="false", cadre="false")}</span><div><b>{t}</b><span>{d}</span></div><span class="tog"></span></div>' for t, d, ob, o in [
      ('Attaquer', 'ATQ contre CA. Arme en main.', 'epee', ''), ('Se cacher', 'DEX (Discrétion) contre la Perception.', 'anneau', ''), ('Fouiller', 'SAG (Perception), difficulté du MJ.', 'parchemin', ''),
      ('Aider', 'Donne l’avantage à un allié.', 'bourse', ''), ('Se désengager', 'Bouger sans attaque d’opportunité.', 'bouclier', ''), ('Esquiver', 'Désavantage aux attaques reçues.', 'bouclier', ''),
      ('Pousser', 'FOR (Athlétisme) contre FOR ou DEX.', 'hache', ''), ('Lancer un sort', 'Selon la classe et les emplacements.', 'orbe', ''), ('Saisir', 'Remplacé par « Pousser » à cette table.', 'couronne', 'off')])
  + '</div><div class="row2" style="align-items: center">' + imp('GameCard', (100, 140), w=100, titre='Se cacher', type='Action', texte='Discrétion contre leur Perception.', valeur='+2', stat='none', icon='oeil', rarete='aucune')
  + '<span class="mut" style="flex: 1">Chaque action active devient une carte dans la main des joueurs, avec son bonus déjà calculé. « Autre… » reste toujours là pour une idée hors des cartes.</span></div>'))
C.append(when(5, head('Une règle maison', 'écrite en français')
  + '<div class="edit"><span class="lbl">Ce que tu écris</span>Les morts-vivants ont peur du feu : quand l’un d’eux prend des dégâts de feu, il est effrayé jusqu’à la fin de son prochain tour.</div>'
  + iff('formal', '<div class="formal on"><span>Quand</span><span>une créature avec l’étiquette <b>mort-vivant</b> subit des dégâts de type <b>feu</b></span><span>Effet</span><span>état <b>Effrayé</b>, 1 tour (jusqu’à la fin de son prochain tour)</span><span>Exception</span><span>aucune : les chefs aussi (dis-le si tu veux les exclure)</span><span>Joueurs</span><span>voient l’état sur le monstre, pas la règle</span></div>'
        '<div class="case"><i>✓</i>Torche contre zombie : effrayé 1 tour</div><div class="case"><i>✓</i>Sort de feu contre trois squelettes : les trois effrayés</div><div class="case"><i class="no">—</i>Feu contre le chef gobelin : rien, ce n’est pas un mort-vivant</div>')
  + iff('notFormal', '<div class="sent"><span class="spin"></span>Le co-MJ traduit ta règle…</div>', True)))
C.append(when(6, head('La création de personnage', 'ce que le créateur des joueurs applique')
  + '<div class="lim">' + ''.join(f'<div class="rl"><span class="lbl">{a}</span><span>{b}</span></div>' for a, b in [
      ('Niveau de départ', '1'), ('Caractéristiques', '27 points · 15 maximum avant bonus du peuple'), ('Peuples', 'nain, elfe, humain, halfelin'),
      ('Classes', 'guerrier, rôdeur, roublard, magicien'), ('Équipement', 'celui de la classe, pas d’achat'), ('Hors limites', 'signalé au joueur, envoyé quand même, tu décides')]) + '</div>'
  + '<div class="secret"><span class="lbl">Ce qui en découle</span><span>Une Force à 18 pour un nain sera signalée en orange au joueur, et par le co-MJ quand tu reliras la fiche (planche « Inviter », moment 4).</span></div>'))
C.append(when(7, head('Tester avant de jouer', 'le co-MJ simule') + '<div class="sim">'
  + '<div class="big"><span class="lbl">Borin contre 2 gobelins</span><b>{{win}}</b>de victoires sur 200 combats</div><div class="big"><span class="lbl">Durée moyenne</span><b>3,1</b>tours</div><div class="big"><span class="lbl">Torche contre zombies</span><b>+18 %</b>de victoires avec ta règle</div></div>'
  + '<div class="bars">' + ''.join(f'<i class="{"hi" if i in (2, 3) else ""}" style="height: {h}%"></i>' for i, h in enumerate([20, 55, 100, 80, 42, 18, 8, 4])) + '</div>'
  + '<div class="chk warn"><i>!</i><span>Ta règle rend le feu très fort dans la crypte : Sef a deux fioles de feu grégeois. Pense à en limiter le nombre.</span></div>'
  + iff('locked', '<div class="ver"><span class="st ok">verrouillée</span><span>Règles de Valombre · version 2. Les parties en cours passent à la version 2 au début de la prochaine session.</span></div>')))

LEFT = ('<section class="panel" style="flex: 1"><div class="ph"><span class="lbl" style="color: #F2F2F2">Règles de Valombre</span><span class="lbl">version {{ver}}</span></div><div class="nav2">'
        + ''.join(f'<span class="{{{{n{i}}}}}">{t}<small>{{{{c{i}}}}}</small></span>' for i, t in enumerate(TABS, 1)) + '</div></section>')
RIGHT = '<section class="panel"><div class="ph"><span class="lbl" style="color: #F2F2F2">Le co-MJ</span></div><div class="co"><div class="note"><span class="lbl">{{coLbl}}</span>{{coTxt}}</div></div></section>'
TOP = top('Les Cendres de Valombre', ['Histoire', 'Règles', 'Cartes', 'Médias', 'Joueurs', 'Sessions'], 'Règles', '<span>{{saveTxt}}</span>')
lap = laptop(TOP, LEFT, ''.join(C), RIGHT, cols='260px 1fr 320px')

M = lambda lbl, sub, act, on='true': f'=[{lbl!r}, {sub!r}, {on}, () => {{ {act} }}]'
ROWS = {
    1: dict(bn=['you', 'D&D 5e (SRD) choisi à la préparation'], main=M('Personnaliser', 'Une copie pour cette campagne', 'this.go(2)'),
            thought='Le SRD me va, mais il y a deux ou trois choses que je fais toujours autrement.',
            why='Le MJ part d’un préréglage complet. Personnaliser crée une copie pour la campagne : on ne casse jamais le préréglage, ni une autre campagne.',
            coLbl='Le co-MJ', coTxt='Ce préréglage couvre tout ce que la campagne utilise. Je te signale ce que tes changements touchent.'),
    2: dict(bn=['you', 'Les statistiques de jeu'], main=M('Compter en cases', 'Mouvement : 9 m → 6 cases', 'this.setState({ cases: true })', '!s.cases'),
            thought='=s.cases ? "Six cases. Mes joueurs ne comptent pas en mètres." : "Les mètres, personne ne les compte à ma table."',
            why='Les cinq statistiques sont celles des gemmes. Le MJ change un nom ou une formule ; la gemme en jeu suit, et le serveur calcule.',
            coLbl='Le co-MJ', coTxt='=s.cases ? "Fait : 1 case = 1,5 m. Les cartes et les gemmes affichent des cases." : "Tu peux aussi garder les mètres et laisser la carte convertir."'),
    3: dict(bn=['you', 'Les difficultés'], main=M('Garder 12 pour « facile »', 'Le reste ne change pas', 'this.go(4)'),
            thought='À 10, mes joueurs réussissaient tout. 12.',
            why='La résolution est lisible en une phrase. Les difficultés sont les quatre que le MJ donnera en jeu d’un clic, sans chercher dans un tableau.',
            coLbl='Le co-MJ', coTxt='À 12 au lieu de 10, un niveau 3 avec +4 réussit 65 % du temps au lieu de 75 %.'),
    4: dict(bn=['you', 'Les actions des joueurs'], main=M('Garder ces 8 actions', '« Saisir » désactivée', 'this.go(5)'),
            thought='« Saisir » m’a toujours ennuyé. Pousser suffit.',
            why='Les actions sont les cartes que les joueurs auront en main, dérivées des règles. Un joueur voit toujours ce qu’il peut faire, avec son bonus.',
            coLbl='Le co-MJ', coTxt='Sans « Saisir », le chef gobelin perd sa prise à la gorge : je lui propose « Pousser » à la place.'),
    5: dict(bn=['=s.formal ? "you" : "wait"', '=s.formal ? "Ta règle est formalisée" : "Le co-MJ lit ta règle…"'], main=M('Ajouter la règle', 'Avec ses trois cas de test', 'this.go(6)', 's.formal'),
            thought='=s.formal ? "C’est exactement ça. Et oui, les chefs aussi." : "Je l’écris comme je la dirais à la table."',
            why='Une règle maison s’écrit en français. Le co-MJ la traduit en règle que le serveur sait juger, avec des cas concrets ; le MJ valide ce qu’il lit.',
            coLbl='Le co-MJ', coTxt='=s.formal ? "Trois cas de test. Dis-moi si un cas te surprend." : "Je cherche les créatures concernées dans ta campagne…"'),
    6: dict(bn=['you', 'La création de personnage'], main=M('Tester les règles', 'Le co-MJ simule des combats', 'this.go(7)'),
            thought='27 points, niveau 1, et je garde le dernier mot sur le reste.',
            why='Ces limites pilotent le créateur des joueurs. Une limite dépassée est signalée au joueur, jamais bloquée : le MJ décide à la validation.',
            coLbl='Le co-MJ', coTxt='Les quatre classes ont leurs cartes de départ prêtes. Rien ne manque pour la création.'),
    7: dict(bn=['=s.locked ? "you" : "warn"', '=s.locked ? "Version 2 verrouillée" : "Un point d’attention"'], main=M('Verrouiller la version 2', 'Elle s’applique à la prochaine session', 'this.setState({ locked: true })', '!s.locked'),
            thought='=s.locked ? "Version 2. Je limite les fioles de Sef dans le butin." : "Le feu devient fort. Je garde la règle, je limite les fioles."',
            why='Avant de jouer, le co-MJ fait tourner des combats simulés avec les nouvelles règles. Le MJ voit l’effet de ses choix avant que ses joueurs le découvrent.',
            coLbl='Le co-MJ', coTxt='Simulé avec les fiches réelles de la table et les monstres de la crypte.'),
}
JS = component(7, ROWS,
               fresh='{ cases: false, formal: false, locked: false }',
               settled='{ cases: m >= 2, formal: m >= 5, locked: m >= 7 }',
               go='if (m > 2) this.setState({ cases: true }); if (m === 5) this.later(() => this.setState({ formal: true }), 2400); if (m > 5) this.setState({ formal: true });',
               vals=f'''mvtName: s.cases ? 'Pas' : 'Mouvement', mvtFormula: s.cases ? 'vitesse ÷ 1,5 m · en cases' : 'vitesse du peuple · en mètres', mvtVal: s.cases ? '5' : '7,5',
      dd1: m >= 3 ? '12' : '10', formal: s.formal, notFormal: !s.formal, locked: s.locked, win: '87 %', ver: s.locked ? '2' : '2 · brouillon',
      saveTxt: s.locked ? 'Version 2 verrouillée' : 'Brouillon enregistré',
      ...Object.fromEntries(Array.from({{ length: 7 }}, (_, i) => [['n' + (i + 1), i + 1 === m ? 'cur' : i + 1 < m ? 'done' : ''], ['c' + (i + 1), i + 1 < m || (i + 1 === 7 && s.locked) ? '✓' : '']]).flat()),''',
               full=(2012, 1020), seul=(1440, 900))
parts = lap + iff('full', side('Romain, le MJ', 'Les règles', 7), True)
board('Regles-MJ', 'Les règles de la campagne', 'rg-', parts, JS, 7, (2012, 1020),
      {"x": 2600, "y": 15600, "w": 2012, "h": 1020, "title": "Règles · le système de Valombre", "is_interactive": True}, css)
