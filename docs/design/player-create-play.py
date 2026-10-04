"""Generate the playable character creator: Marc builds Borin on his phone (Creer-Joueur.dc.html).

Eight moments, after Marc opened the invite link (Inviter-MJ, moment 2): people, body, outfit and
weapon, colours and name, class, abilities, backstory, send. The pieces come from the universe
pack; the rules steps come from the campaign's rule system. Target: under two minutes.
Run from the repository root.
"""
import sys
sys.path.insert(0, 'docs/design')
from kit import imp, when, iff, head, note, chk, cb, phone, side, component, board
from avatar import avatar, palettes_js, OPTIONS, PALETTES, AV_CSS

css = AV_CSS + r'''
.ppl{display:grid;grid-template-columns:1fr 1fr;gap:8px}
.pp{display:flex;flex-direction:column;align-items:center;gap:4px;padding:10px 6px 8px;border-radius:12px;background:#141414;border:1.5px solid #2C2C2C;font-size:11px;color:#A3A3A3;text-align:center;line-height:1.35}
.pp > b{font-family:'Cinzel',serif;font-size:15px;color:#F2F2F2}
.pp.on{background:#EDEDED;border-color:#0A0A0A;color:#5A5A5A;box-shadow:0 3px 0 #8A8A8A}
.pp.on > b{color:#0A0A0A}
.avs{display:grid;place-items:center;height:236px;border-radius:16px;background:radial-gradient(50% 22% at 50% 88%,rgba(255,255,255,.12),transparent),#141414;border:1px solid #2C2C2C;position:relative;flex:none}
.avs > .pk{position:absolute;left:10px;top:10px;font-size:10px;font-weight:700;letter-spacing:.12em;text-transform:uppercase;color:#8C8C8C}
.ltabs{display:flex;gap:4px;padding:3px;border-radius:10px;background:#0D0D0D;border:1px solid #2C2C2C}
.ltabs > span{flex:1;text-align:center;font-size:12px;font-weight:700;padding:7px 0;border-radius:7px;color:#8C8C8C}
.ltabs > span.on{background:#EDEDED;color:#0A0A0A}
.swr{display:flex;align-items:center;gap:10px}
.swr > .lbl{width:64px;flex:none}
.dice{font:inherit;display:flex;align-items:center;justify-content:center;gap:8px;height:44px;border-radius:12px;border:1.5px dashed #8C8C8C;background:#0D0D0D;color:#F2F2F2;font-weight:700;font-size:14px;cursor:pointer;flex:none}
.field{display:flex;flex-direction:column;gap:6px}
.field > span:last-child{display:flex;align-items:center;justify-content:space-between;padding:12px 14px;border-radius:10px;background:#0D0D0D;border:1px solid #3A3A3A;font-size:16px}
.field > span:last-child > small{font-size:11px;color:#8C8C8C}
.cls{display:grid;grid-template-columns:1fr 1fr;gap:8px}
.cl{display:flex;flex-direction:column;gap:2px;padding:10px;border-radius:12px;background:#141414;border:1.5px solid #2C2C2C;font-size:12px;color:#A3A3A3}
.cl > b{font-family:'Cinzel',serif;font-size:15px;color:#F2F2F2}
.cl.on{background:#EDEDED;border-color:#0A0A0A;color:#5A5A5A;box-shadow:0 3px 0 #8A8A8A}
.cl.on > b{color:#0A0A0A}
.pts{display:flex;justify-content:space-between;align-items:baseline;padding:10px 12px;border-radius:10px;background:#121212;font-size:13px}
.pts > b{font-size:18px}
.abr{display:grid;grid-template-columns:44px 1fr 28px 34px 28px;align-items:center;gap:8px;padding:6px 10px;border-radius:10px;background:#121212;border:1px solid transparent;font-size:12px;color:#A3A3A3}
.abr > b{font-size:12px;letter-spacing:.12em;color:#F2F2F2}
.abr > strong{font-size:18px;text-align:center;color:#F2F2F2}
.abr > i{font-style:normal;width:28px;height:28px;border-radius:8px;display:grid;place-items:center;background:#1E1E1E;border:1px solid #3A3A3A;color:#F2F2F2;font-weight:800}
.abr.warn{border:1.5px solid #FF9F1C}
.abr.warn > strong{color:#FF9F1C}
.warnbox{display:flex;flex-direction:column;gap:4px;padding:10px 12px;border-radius:10px;border:1.5px solid #FF9F1C;background:#0A0A0A;font-size:12px;line-height:1.45;color:#D4D4D4}
.warnbox > b{color:#FF9F1C}
.q{display:flex;flex-direction:column;gap:4px;padding:10px 12px;border-radius:10px;background:#121212;font-size:13px;line-height:1.4}
.q > small{font-size:10px;font-weight:700;letter-spacing:.14em;text-transform:uppercase;color:#8C8C8C}
.story{padding:12px 14px;border-radius:12px;background:#0D0D0D;border:1px solid #3A3A3A;font-size:13px;line-height:1.5;color:#D4D4D4}
.story > .lbl{display:block;margin-bottom:6px}
.minihand{display:flex;gap:6px}
.timer{margin-left:auto;font-size:12px;font-weight:700;font-variant-numeric:tabular-nums;opacity:.75}
'''

PAL = palettes_js()


def chips(layer, act):
    return '<div class="chips">' + ''.join(f'<button type="button" class="chip {{{{{layer}Cls_{k}}}}}" onClick="{{{{{act}_{k}}}}}">{n}</button>' for k, n in OPTIONS[layer]) + '</div>'


def swatches(layer):
    return f'<div class="swr"><span class="lbl">{ {"skin": "Peau", "hair": "Poils", "outfit": "Tenue"}[layer] }</span><div class="chips">' + ''.join(
        f'<button type="button" class="sw {{{{{layer}Sw_{k}}}}}" style="background: {c}" onClick="{{{{pick_{layer}_{k}}}}}" aria-label="{k}"></button>' for k, c in PALETTES[layer].items()) + '</div></div>'


def avstage(label, px=8):
    return f'<div class="avs"><span class="pk">{label}</span>{avatar(px)}</div>'


def ltabs(on):
    return '<div class="ltabs">' + ''.join(f'<span class="{"on" if t == on else ""}">{t}</span>' for t in ('Corps', 'Tenue', 'Couleurs')) + '</div>'


def ban(step, txt):
    return f'<span>{txt}</span><span class="timer">{{{{clock}}}}</span>' if step else txt


PEOPLE = [('borin', 'Nain', 'Solide. Voit dans le noir.'), ('lyra', 'Elfe', 'Agile. Ne dort jamais vraiment.'),
          ('mara', 'Humain', 'Un peu de tout, apprend vite.'), ('sef', 'Halfelin', 'Petit, chanceux, discret.')]

P = {}
P[1] = ('you', ban(1, 'Ton personnage · 1 sur 4'),
        '<div class="dots"><i class="on"></i><i></i><i></i><i></i></div><span class="ttl" style="font-size: 22px">Ton peuple</span>'
        '<div class="ppl">' + ''.join(f'<div class="pp {{{{pp_{p}}}}}">{imp("Perso", (60, 78), perso=p, px=3, etat="aucun")}<b>{n}</b><span>{d}</span></div>' for p, n, d in PEOPLE) + '</div>'
        + iff('dwarf', '<div class="q"><small>Ce que ça change en jeu</small><span>+2 en Constitution · vision dans le noir · résistant au poison. Vitesse un peu plus lente.</span></div>'),
        cb('Continuer', 'Ton allure', act='next'))
P[2] = ('you', ban(1, 'Ton personnage · 1 sur 4'),
        '<div class="dots"><i class="on"></i><i></i><i></i><i></i></div>' + avstage('Ton nain') + ltabs('Corps')
        + swatches('skin') + '<span class="lbl">Barbe</span>' + chips('beard', 'pick_beard'),
        cb('Continuer', 'Tenue et arme', act='next'))
P[3] = ('you', ban(1, 'Ton personnage · 1 sur 4'),
        '<div class="dots"><i class="on"></i><i></i><i></i><i></i></div>' + avstage('Ton nain') + ltabs('Tenue')
        + '<span class="lbl">Sur la tête</span>' + chips('head', 'pick_head') + '<span class="lbl">Dans la main</span>' + chips('weapon', 'pick_weapon'),
        cb('Continuer', 'Couleurs et nom', act='next'))
P[4] = ('you', ban(1, 'Ton personnage · 1 sur 4'),
        '<div class="dots"><i class="on"></i><i></i><i></i><i></i></div>' + avstage('{{avName}}') + ltabs('Couleurs')
        + swatches('hair') + swatches('outfit')
        + '<button type="button" class="dice" onClick="{{shuffle}}">🎲 Au hasard</button>'
        + '<div class="field"><span class="lbl">Son nom</span><span>{{avName}}<small>{{nameHint}}</small></span></div>',
        cb('Continuer', 'Sa classe', act='next'))
P[5] = ('you', ban(2, 'Ton personnage · 2 sur 4'),
        '<div class="dots"><i class="on"></i><i class="on"></i><i></i><i></i></div><span class="ttl" style="font-size: 22px">Ce que tu sais faire</span>'
        '<div class="cls">' + ''.join(f'<div class="cl {"on" if n == "Guerrier" else ""}"><b>{n}</b>{d}</div>' for n, d in [('Guerrier', 'frapper, encaisser'), ('Rôdeur', 'tirer, pister'), ('Roublard', 'se cacher, crocheter'), ('Magicien', 'sorts, savoir')]) + '</div>'
        '<span class="lbl">Tes cartes de départ · guerrier</span><div class="minihand">'
        + ''.join(imp('GameCard', (100, 140), w=100, **c) for c in [
            dict(titre='Hache d’armes', type='Arme', texte='1d12 + 3 dégâts.', valeur='+5', stat='atq', icon='epee', rarete='commune'),
            dict(titre='Second souffle', type='Compétence', texte='Soigne 1d10 + 1.', valeur='d10', stat='pv', icon='sort', rarete='peu-commune'),
            dict(titre='Fouiller', type='Sagesse', texte='Chercher, observer.', valeur='+1', stat='none', icon='oeil', rarete='commune')])
        + '</div><span class="mut">Les cartes viennent des règles de la campagne : D&amp;D 5e (SRD).</span>',
        cb('Continuer', 'Tes caractéristiques', act='next'))


def abrow(k, label, val_hole, extra=''):
    return f'<div class="abr {extra}"><b>{k}</b><span>{label}</span><i>−</i><strong>{val_hole}</strong><i>+</i></div>'


P[6] = ('you', ban(3, 'Ton personnage · 3 sur 4'),
        '<div class="dots"><i class="on"></i><i class="on"></i><i class="on"></i><i></i></div><span class="ttl" style="font-size: 22px">Tes caractéristiques</span>'
        '<div class="pts"><span>Répartition conseillée pour un guerrier nain</span><b>{{ptsLeft}}</b></div>'
        + '<div class="abr {{forCls}}"><b>FOR</b><span>Force · frapper, porter</span><i>−</i><strong>{{forVal}}</strong><button type="button" class="chip" style="width: 28px; height: 28px; padding: 0; justify-content: center" onClick="{{morefor}}">+</button></div>'
        + abrow('DEX', 'Dextérité · esquiver', '10') + abrow('CON', 'Constitution · tenir', '16') + abrow('INT', 'Intelligence · savoir', '8')
        + '<div class="abr"><b>SAG</b><span>Sagesse · remarquer</span><i>−</i><strong>{{sagVal}}</strong><i>+</i></div>' + abrow('CHA', 'Charisme · convaincre', '8')
        + iff('over', '<div class="warnbox"><b>18 dépasse la limite des règles</b>Au niveau 1, la Force s’arrête à 17 pour un nain. Tu peux l’envoyer quand même : Romain décidera.</div>'),
        cb('Continuer', 'Son histoire', act='next'))
P[7] = ('you', ban(4, 'Ton personnage · 4 sur 4'),
        '<div class="dots"><i class="on"></i><i class="on"></i><i class="on"></i><i class="on"></i></div><span class="ttl" style="font-size: 22px">Son histoire</span>'
        '<div class="q"><small>D’où vient-il ?</small><span>De la mine d’argent de Valombre.</span></div>'
        '<div class="q"><small>Qui a-t-il perdu ?</small><span>Son frère Dorn, dans la mine.</span></div>'
        '<div class="q"><small>Que cherche-t-il ?</small><span>Pourquoi la mine paie encore.</span></div>'
        + iff('notWritten', '<span class="mut">Trois réponses courtes suffisent. Le co-MJ peut en faire un paragraphe, que tu modifies.</span>', True)
        + iff('written', '<div class="story"><span class="lbl">Écrit avec le co-MJ · modifiable</span>Borin a travaillé vingt ans dans la mine d’argent. Son frère Dorn y est descendu un soir et n’est jamais remonté. Borin veut savoir pourquoi la mine paie encore.</div>'),
        iff('notWritten', cb('Écrire avec le co-MJ', 'À partir de tes trois réponses', act='write'), True) + iff('written', cb('Continuer', 'Relire et envoyer', act='next')))
P[8] = ('you', ban(4, 'Prêt à partir'),
        '<div class="pstage" style="align-items: center">' + avatar(4, 'sm') + '<div style="display: flex; flex-direction: column; gap: 4px"><span class="ttl" style="font-size: 22px">Borin</span><span style="font-size: 12px; color: #A3A3A3">nain guerrier · niveau 1</span></div></div>'
        '<div class="gems">' + ''.join(f'<span class="gem">{imp("Gemme", (40, 40), stat=s, valeur=v, taille=40, anim="false")}{n}</span>' for s, v, n in [('pv', '13', 'Vie'), ('ca', '18', 'Armure'), ('atq', '+6', 'Hache'), ('mvt', '5', 'Pas')]) + '</div>'
        '<div class="warnbox"><b>1 point à voir avec le MJ</b>Force 18 au lieu de 17. Romain peut l’accepter ou te demander de changer.</div>'
        '<div class="story" style="font-size: 12px">Borin a travaillé vingt ans dans la mine d’argent. Son frère Dorn y est descendu un soir…</div>'
        + iff('ended', '<div class="sent"><span class="spin"></span>Romain regarde ton personnage…</div>'),
        iff('notEnded', cb('Envoyer au MJ', 'Créé en {{clock}}', act='send'), True) + iff('ended', '<span class="mut" style="text-align: center">Tu auras une notification quand il aura répondu.</span>'))

ph = phone(P)

M = lambda lbl, sub, act, on='true': f'=[{lbl!r}, {sub!r}, {on}, () => {{ {act} }}]'
ROWS = {
    1: dict(bn=['you', 'Marc choisit son peuple'], main=M('Nain', '', 'this.go(2)'),
            thought='Un nain. Évidemment un nain.',
            why='Le peuple vient des règles de la campagne : chaque carte dit en une ligne ce qu’il change en jeu. Les sprites viennent du pack de l’univers.'),
    2: dict(bn=['you', 'Il façonne son corps'], main=M('', '', ''),
            thought='Peau hâlée, barbe longue. Il me ressemble déjà.',
            why='Le créateur en couches : chaque pièce est un calque du pack de l’univers, posé sur les autres. Ce qu’on touche change le personnage tout de suite.'),
    3: dict(bn=['you', 'Il choisit sa tenue et son arme'], main=M('', '', ''),
            thought='Le casque à cornes. Et une hache, pour la mine.',
            why='Tenue et arme sont des pièces, pas des règles : l’arme dessinée ne change pas les dégâts. La classe décidera de ce qu’il sait faire.'),
    4: dict(bn=['you', 'Couleurs et nom'], main=M('', '', ''),
            thought='J’ai appuyé trois fois sur « au hasard » pour rire. Je reviens au roux.',
            why='« Au hasard » aide ceux qui ne savent pas quoi choisir. Le nom est proposé par le co-MJ dans le style de l’univers ; on le remplace d’un doigt.'),
    5: dict(bn=['you', 'Sa classe'], main=M('', '', ''),
            thought='Guerrier. Je veux frapper fort et protéger les autres.',
            why='Une classe se choisit par ce qu’on veut faire, pas par son nom. Les cartes de départ montrent tout de suite ce qu’il aura en main à la table.'),
    6: dict(bn=['warn', 'Il dépasse une limite'], main=M('', '', ''),
            thought='18 en Force, c’est mieux que 17, non ?',
            why='La répartition est préremplie. Une limite dépassée est expliquée, jamais bloquée : le joueur peut envoyer quand même, le MJ a le dernier mot.'),
    7: dict(bn=['you', 'Son histoire'], main=M('', '', ''),
            thought='Mon frère disparu dans la mine. Ça me plaît.',
            why='Trois questions courtes au lieu d’une page blanche. Le co-MJ en fait un paragraphe que le joueur garde ou modifie ; le MJ s’en servira pour ses accroches.'),
    8: dict(bn=['=s.ended ? "wait" : "you"', '=s.ended ? "Envoyé à Romain" : "Il relit et envoie"'], main=M('', '', ''),
            thought='=s.ended ? "Moins de deux minutes. Plus qu’à attendre Romain." : "Il y a un point orange. Tant pis, Romain verra."',
            why='Le récapitulatif montre ce qui partira au MJ, avec le point à discuter. La suite se passe sur la planche « Inviter » : validation ou renvoi avec un mot.'),
}
# The phone carries its own buttons: the laptop-style footer of the kit is not used here.
JS = component(8, ROWS,
               fresh="{ skin: 'hale', hair: 'roux', outfit: 'bleu', beard: 'longue', head: 'casque', weapon: 'hache', forv: 17, written: false, rolls: 0, people: '' }",
               settled="{ people: 'borin', forv: m >= 6 ? 18 : 17, written: m >= 7, ended: m >= 8 }",
               go="if (m > 1) this.setState({ people: 'borin' }); if (m >= 7) this.setState({ forv: 18 }); if (m === 1) this.later(() => this.setState({ people: 'borin' }), 1600);",
               pre=f"const PAL = {PAL}; const pick = (k, val) => () => this.setState({{ [k]: val, rolls: s.rolls }});",
               vals='''avVars: PAL.skin[s.skin] + '; ' + PAL.hair[s.hair] + '; ' + PAL.outfit[s.outfit],
      smVars: PAL.skin[s.skin] + '; ' + PAL.hair[s.hair] + '; ' + PAL.outfit[s.outfit],
      dwarf: s.people === 'borin', pp_borin: s.people === 'borin' ? 'on' : '', pp_lyra: '', pp_mara: '', pp_sef: '',
      avName: 'Borin', nameHint: s.rolls ? 'proposé · ' + ['Borin', 'Thrain', 'Dagna', 'Borin'][s.rolls % 4] + ' ?' : 'proposé par le co-MJ',
      forVal: String(s.forv), sagVal: '12', over: s.forv > 17, forCls: s.forv > 17 ? 'warn' : '', ptsLeft: s.forv > 17 ? '−1 point' : '0 point restant',
      written: s.written, notWritten: !s.written,
      clock: ['0:12', '0:31', '0:44', '0:58', '1:09', '1:22', '1:36', '1:48'][m - 1],
      next: () => this.go(m + 1), morefor: () => this.setState({ forv: 18 }), write: () => this.setState({ written: true }),
      send: () => this.end(),
      shuffle: () => { const r = s.rolls + 1; const o = ['rouge', 'vert', 'violet', 'bleu'][r % 4], h = ['noir', 'blanc', 'brun', 'roux'][r % 4]; this.setState({ rolls: r, outfit: o, hair: h, beard: ['tressee', 'courte', 'longue'][r % 3] }); },
      ...Object.fromEntries(['skin', 'hair', 'outfit'].flatMap((k) => Object.keys(PAL[k]).flatMap((n) => [[k + 'Sw_' + n, s[k] === n ? 'on' : ''], ['pick_' + k + '_' + n, pick(k, n)]]))),
      ...Object.fromEntries([['beard', ['longue', 'tressee', 'courte']], ['head', ['casque', 'capuche', 'nu']], ['weapon', ['hache', 'marteau', 'epee']]].flatMap(([k, ns]) => ns.flatMap((n) => [[k + 'Cls_' + n, s[k] === n ? 'on' : ''], ['pick_' + k + '_' + n, pick(k, n)], ['av_' + k + '_' + n, s[k] === n], ['sm_' + k + '_' + n, s[k] === n]]))),''',
               full=(1018, 964), seul=(390, 844), carry=('skin', 'hair', 'outfit', 'beard', 'head', 'weapon', 'rolls'))

parts = ph + iff('full', side('Marc, le joueur', 'Créer son personnage', 8), True)
board('Creer-Joueur', 'Créer son personnage', 'cr-', parts, JS, 8, (1018, 964),
      {"x": 0, "y": 13600, "w": 1018, "h": 964, "title": "Créer · le personnage de Marc", "is_interactive": True}, css)
