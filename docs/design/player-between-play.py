"""Generate the player's time between sessions: Marc after session 3 and before session 4 (Entre-Joueur.dc.html).

Six moments on the phone: the end of the evening, levelling up, the published recap, the campaign
chronicle, the character sheet outside play, and the reminder that leads back to the lobby.
Run from the repository root.
"""
import sys
sys.path.insert(0, 'docs/design')
from kit import imp, when, iff, head, note, chk, cb, phone, side, component, board

css = r'''
.xp{display:flex;flex-direction:column;gap:6px;padding:12px 14px;border-radius:12px;background:#141414;border:1px solid #2C2C2C}
.xp > .bar{height:10px;border-radius:5px;background:#0A0A0A;border:1px solid #3A3A3A;overflow:hidden}
.xp > .bar > i{display:block;height:100%;background:#F2F2F2;transition:width 1.2s ease-out}
.xp > span{display:flex;justify-content:space-between;font-size:12px;color:#A3A3A3}
.li{display:flex;align-items:center;gap:12px;font-size:13px;padding:8px;border-radius:10px;background:#141414;border:1px solid #2C2C2C;line-height:1.4}
.li > em{margin-left:auto;font-style:normal;font-size:11px;color:#8C8C8C;white-space:nowrap}
.lvl{display:flex;flex-direction:column;align-items:center;gap:8px;padding:16px;border-radius:16px;background:#EDEDED;color:#0A0A0A;box-shadow:0 4px 0 #8A8A8A;text-align:center}
.lvl > .lbl{color:#5A5A5A}
.opt2{display:grid;grid-template-columns:1fr 1fr;gap:8px}
.op{font:inherit;display:flex;flex-direction:column;align-items:center;gap:6px;padding:12px 8px;border-radius:12px;background:#141414;border:1.5px solid #2C2C2C;color:#D4D4D4;font-size:12px;cursor:pointer;text-align:center}
.op > b{font-family:'Cinzel',serif;font-size:15px;color:#F2F2F2}
.op.on{background:#EDEDED;border-color:#0A0A0A;color:#5A5A5A;box-shadow:0 3px 0 #8A8A8A}
.op.on > b{color:#0A0A0A}
.tl{display:flex;flex-direction:column;border-left:2px solid #2C2C2C;margin-left:6px}
.ev{position:relative;display:flex;flex-direction:column;gap:3px;padding:0 0 14px 18px;font-size:13px;line-height:1.45;color:#D4D4D4}
.ev::before{content:'';position:absolute;left:-7px;top:4px;width:12px;height:12px;border-radius:3px;background:#3A3A3A;box-shadow:0 0 0 3px #0C0C0C}
.ev.key::before{background:#F2F2F2}
.ev > time{font-size:10px;font-weight:700;letter-spacing:.14em;text-transform:uppercase;color:#6E6E6E}
.ev > b{font-family:'Cinzel',serif;font-size:15px;color:#F2F2F2}
.thumbs{display:flex;gap:6px}
.thumbs > i{width:64px;height:40px;border-radius:6px;background:repeating-linear-gradient(135deg,#3A3A3A 0 2px,#262626 2px 7px);border:1px solid #2C2C2C}
.wear{display:grid;grid-template-columns:repeat(5,1fr);gap:6px}
.detail{display:flex;gap:12px;align-items:flex-start;padding:12px;border-radius:12px;background:#141414;border:1.5px solid #F2F2F2}
.detail > div{display:flex;flex-direction:column;gap:4px;font-size:12px;line-height:1.45;color:#D4D4D4}
.detail > div > b{font-family:'Cinzel',serif;font-size:16px;color:#F2F2F2}
.days{display:flex;gap:6px}
.day{display:flex;flex-direction:column;align-items:center;gap:2px;flex:1;padding:10px 0;border-radius:12px;background:#141414;border:1.5px solid #2C2C2C;font-size:11px;color:#A3A3A3}
.day > b{font-family:'Cinzel',serif;font-size:22px;color:#F2F2F2}
.day.on{background:#EDEDED;border-color:#0A0A0A;color:#5A5A5A;box-shadow:0 3px 0 #8A8A8A}
.day.on > b{color:#0A0A0A}
'''

borin = lambda px=4: imp('Perso', (20 * px, 26 * px), perso='borin', px=px, etat='aucun')
P = {}
P[1] = ('listen', 'Fin de la session 3',
        '<span class="lbl">Ce soir · 2 h 40 de jeu</span><span class="ttl" style="font-size: 26px; line-height: 1.1">La crypte des Valombre</span>'
        '<p class="nar" style="font-size: 19px">« Vous ressortez à l’aube, la page arrachée dans la poche de Borin. »</p>'
        '<div class="xp"><span><b style="color: #F2F2F2">Expérience</b><span>{{xpTxt}}</span></span><div class="bar"><i style="width: {{xpW}}"></i></div></div>'
        '<span class="lbl">Ce que tu as gagné</span>'
        '<div class="li">' + imp('Objet', (40, 40), objet='hache', rarete='leg', taille=40, pips='false') + '<span><b>Hache des Valombre</b> · équipée</span><em>légendaire</em></div>'
        '<div class="li">' + imp('Objet', (40, 40), objet='bourse', rarete='commune', taille=40, pips='false') + '<span><b>34 pièces d’or</b></span><em>partagées</em></div>'
        '<span class="mut">Romain relit le récapitulatif. Tu le recevras demain.</span>',
        iff('lvlUp', cb('Passer au niveau 4', 'Ça prend trente secondes', act='next')))
P[2] = ('you', 'Borin passe au niveau 4',
        '<div class="lvl"><span class="lbl">Niveau 4</span>' + borin(5) + '<span class="ttl" style="font-size: 22px">Borin</span></div>'
        '<span class="lbl">Tes points de vie</span><div class="opt2">'
        '<button type="button" class="op {{hpRoll}}" onClick="{{rollHp}}"><b>Lancer le dé</b>' + imp('De', (44, 44), de='d10', valeur=7, taille=44, anim='fixe', couleur='pv') + '<span>{{rollTxt}}</span></button>'
        '<button type="button" class="op {{hpAvg}}" onClick="{{avgHp}}"><b>La moyenne</b><span style="font-size: 26px; font-weight: 700; color: inherit">+9</span><span>6 + Constitution, sans risque</span></button></div>'
        '<span class="lbl">Ta nouvelle carte · guerrier niveau 4</span><div style="display: flex; gap: 12px; align-items: center">'
        + imp('GameCard', (100, 140), w=100, titre='Plus fort', type='Amélioration', texte='+1 Force, +1 Constitution.', valeur='+2', stat='none', icon='bouclier', rarete='rare')
        + '<span class="mut">Proposée par les règles pour un guerrier qui frappe. Tu peux la changer avant jeudi.</span></div>',
        cb('C’est noté', 'Borin a {{hpMax}} PV', act='next'))
P[3] = ('listen', 'Romain a publié le récap',
        '<span class="lbl">Session 3 · relu par Romain</span><span class="ttl" style="font-size: 24px">Précédemment…</span>'
        '<p class="nar" style="font-size: 18px">Sous l’autel de la crypte, Borin a trouvé la page arrachée du registre. Les Valombre y gardaient « la porte nord ». Deux gobelins défendaient la salle ; la hache de la famille a changé de mains.</p>'
        '<span class="lbl">Ce que vous savez</span>'
        '<div class="li">' + imp('Objet', (36, 36), objet='parchemin', rarete='commune', taille=36, pips='false') + '<span>La page arrachée nomme la porte nord.</span></div>'
        '<div class="li">' + imp('Objet', (36, 36), objet='anneau', rarete='peu', taille=36, pips='false') + '<span>La bague du gardien porte un corbeau.</span></div>'
        '<span class="lbl">Ce qui reste ouvert</span><div class="li"><span>Qui a laissé les cendres tièdes sur l’autel ?</span></div>',
        cb('Lire la chronique', 'Les trois sessions', act='next', dark=True))
P[4] = ('listen', 'La chronique de Valombre',
        '<div class="tl">'
        '<div class="ev"><time>Session 1 · 19 sept.</time><b>Une ville qui enterre deux fois</b><span>Arrivée à Valombre. Le maire ment sur la mine.</span></div>'
        '<div class="ev"><time>Session 2 · 26 sept.</time><b>La mine d’argent</b><span>La lampe de Dorn, gravée à son nom. Borin ne dit rien.</span><span class="thumbs"><i></i><i></i></span></div>'
        '<div class="ev key"><time>Session 3 · 3 oct.</time><b>La crypte des Valombre</b><span>La page arrachée. La hache légendaire. Les oiseaux de Lyra ont fui la colline.</span><span class="thumbs"><i></i><i></i><i></i></span></div>'
        '</div><span class="mut">La chronique ne garde que ce que toute la table a vu. Les illustrations sont celles montrées en jeu.</span>',
        cb('Voir Borin', 'Ta fiche', act='next', dark=True))
P[5] = ('listen', 'Borin · niveau 4',
        '<div class="pstage">' + borin(4) + '<div style="display: flex; flex-direction: column; gap: 6px; flex: 1"><span class="ttl" style="font-size: 20px">Borin</span>'
        + imp('Coeurs', (151, 16), pv=41, max=41, coeurs=8, px=2, anim='false') + '<span class="mut">nain guerrier · 41 PV</span></div></div>'
        '<div class="gems">' + ''.join(f'<span class="gem">{imp("Gemme", (36, 36), stat=s, valeur=v, taille=36, anim="false")}{n}</span>' for s, v, n in [('ca', '18', 'Armure'), ('atq', '+6', 'Hache'), ('mvt', '5', 'Pas'), ('ini', '+0', 'Init.')]) + '</div>'
        '<span class="lbl">Sa main · touche une carte</span><div class="hand" style="gap: 6px">'
        + ''.join(imp('GameCard', (76, 106), w=76, **c) for c in [
            dict(titre='Hache des Valombre', type='Arme', texte='1d12 + 4.', valeur='+6', stat='atq', icon='epee', rarete='legendaire'),
            dict(titre='Second souffle', type='Compétence', texte='Soigne 1d10 + 2.', valeur='d10', stat='pv', icon='sort', rarete='peu-commune'),
            dict(titre='Fougue', type='Compétence', texte='Une action de plus.', valeur='', stat='none', icon='epee', rarete='rare'),
            dict(titre='Fouiller', type='Sagesse', texte='Chercher.', valeur='+1', stat='none', icon='oeil', rarete='commune')]) + '</div>'
        '<div class="detail"><div><b>Hache des Valombre</b><span>Arme légendaire. 1d12 + 4 dégâts, au corps à corps. Les morts-vivants touchés reculent d’une case.</span><span style="color: #8C8C8C">Trouvée sous l’autel · session 3</span></div></div>',
        '')
P[6] = ('you', '=s.ended ? "Ce soir, 20 h 30" : "Session 4 : tu es libre quand ?"',
        iff('notEnded', '<span class="lbl">Romain propose</span><div class="days">' + ''.join(f'<div class="day {{{{d{d}}}}}"><span>{j}</span><b>{d}</b><span>oct.</span></div>' for j, d in [('mer.', '9'), ('jeu.', '10'), ('ven.', '11')]) + '</div>'
            '<div class="ans"><b>Camille</b><span>jeu. · ven.</span></div><div class="ans"><b>Hugo</b><span>jeudi</span></div>'
            '<span class="mut">Romain fixe la date quand tout le monde a répondu.</span>', True)
        + iff('ended', '<div class="toast"><span class="lbl">Dans une heure</span><b class="ttl" style="font-size: 20px">Session 4 · jeudi 10 octobre</b><span style="font-size: 12px">Le salon ouvre à 20 h 15. Prépare tes écouteurs.</span></div>'
              '<div class="pstage">' + borin(4) + '<span style="font-size: 13px; color: #D4D4D4; line-height: 1.5">Borin est prêt : niveau 4, la hache des Valombre en main.</span></div>'),
        iff('notEnded', cb('Envoyer mes dispos', 'Jeudi', act='send'), True) + iff('ended', cb('Rejoindre le salon', 'La suite sur « Jouer · la soirée de Marc »')))

# Banner holes in the phone: replace JS-expression banners with holes.
P[6] = ('you', '{{b6}}') + P[6][2:]
ph = phone(P, nav={1: 'Jeu', 2: 'Perso', 3: 'Journal', 4: 'Journal', 5: 'Perso', 6: 'Jeu'})

M = lambda lbl, sub, act, on='true': f'=[{lbl!r}, {sub!r}, {on}, () => {{ {act} }}]'
NO = M('', '', '')
ROWS = {
    1: dict(bn=['you', 'La soirée se termine'], main=NO, thought='=s.xp >= 900 ? "Niveau 4 ! Je savais que ce gobelin valait le coup." : "On a fini tard. Combien d’expérience ?"',
            why='La fin de soirée arrive sur le téléphone tout de suite : expérience, butin gagné. Le récit, lui, attend que le MJ ait relu le récapitulatif.'),
    2: dict(bn=['you', 'Il monte de niveau'], main=NO, thought='=s.hp ? "Je prends le risque du dé. 7, pas mal." : "Lancer ou prendre la moyenne ?"',
            why='Monter de niveau tient en un écran : un seul vrai choix (dé ou moyenne), le reste vient des règles et arrive tout seul dans sa main.'),
    3: dict(bn=['you', 'Le lendemain, le récap'], main=NO, thought='La porte nord… Il faut qu’on y retourne.',
            why='Le « Précédemment… » est écrit par le co-MJ et relu par le MJ avant de partir. Il ne contient que ce que les joueurs ont vu, et ce qui reste ouvert.'),
    4: dict(bn=['you', 'La chronique'], main=NO, thought='Dorn… Je n’avais pas fait le lien avec la mine.',
            why='La chronique s’allonge d’une entrée par session : un titre, deux lignes, les images montrées en jeu. Elle sert aussi à qui rate une soirée.'),
    5: dict(bn=['you', 'Sa fiche, hors session'], main=NO, thought='Les morts-vivants reculent… Elle est faite pour la crypte, cette hache.',
            why='Hors session, la fiche se lit sans rien casser : on touche une carte pour voir ce qu’elle fait et d’où elle vient. Rien ne se modifie sans le MJ.'),
    6: dict(bn=['=s.ended ? "wait" : "you"', '=s.ended ? "Jeudi soir, le rappel" : "La prochaine date"'], main=NO,
            thought='=s.ended ? "Le salon ouvre dans un quart d’heure. Les écouteurs !" : "Jeudi, c’est bon pour moi."',
            why='Les disponibilités se donnent d’un doigt ; le MJ fixe la date. Le jour venu, le rappel mène droit au salon (planche « Jouer », moment 1).'),
}
JS = component(6, ROWS,
               fresh="{ xp: 620, hp: '', picked: '' }",
               settled="{ xp: 900, hp: m >= 2 ? 'roll' : '', ended: m >= 6 }",
               go="if (m === 1) this.later(() => this.setState({ xp: 900 }), 900); if (m > 2) this.setState({ hp: 'roll', xp: 900 });",
               vals='''xpTxt: s.xp >= 900 ? '+280 · niveau 4 atteint !' : '2 450 / 2 700', xpW: Math.min(100, s.xp / 9) + '%', lvlUp: s.xp >= 900,
      hpRoll: s.hp === 'roll' ? 'on' : '', hpAvg: s.hp === 'avg' ? 'on' : '', rollTxt: s.hp === 'roll' ? '7 + 3 = +10' : 'd10 + Constitution',
      hpMax: s.hp === 'avg' ? '40' : '41', rollHp: () => this.setState({ hp: 'roll' }), avgHp: () => this.setState({ hp: 'avg' }),
      b6: s.ended ? 'Ce soir, 20 h 30' : 'Session 4 : tu es libre quand ?', d9: '', d10: 'on', d11: '',
      next: () => this.go(m + 1), send: () => this.end(),''',
               full=(1018, 964), seul=(390, 844))

parts = ph + iff('full', side('Marc, le joueur', 'Entre deux sessions', 6), True)
board('Entre-Joueur', 'Entre deux sessions', 'es-', parts, JS, 6, (1018, 964),
      {"x": 2400, "y": 13600, "w": 1018, "h": 964, "title": "Entre deux · Marc entre les sessions", "is_interactive": True}, css)
