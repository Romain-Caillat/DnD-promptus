import json
css = open('docs/design/player-evening.css').read()
css += r'''
.ph{flex:none}
.cb{font:inherit;display:block;width:100%;box-sizing:border-box;border:0;padding:6px;border-radius:12px;background:#EDEDED;color:#0A0A0A;text-align:left;cursor:pointer;box-shadow:0 4px 0 #8A8A8A,0 7px 0 #5A5A5A,0 14px 24px rgba(0,0,0,.5);transform-origin:50% 100%;transition:transform .1s,box-shadow .1s}
.cb .in{display:flex;align-items:center;gap:12px;border:1.5px solid #0A0A0A;border-radius:8px;padding:8px 12px;min-height:40px}
.cb .t{display:block;font-family:'Cinzel',Georgia,serif;font-weight:800;font-size:18px;line-height:1.05}
.cb .s{display:block;font-size:11px;font-weight:600;color:#5A5A5A;margin-top:3px}
.cb .chev{margin-left:auto;font-family:'Cinzel',serif;font-weight:800;font-size:22px}
.cb:active{transform:perspective(500px) rotateX(14deg) translateY(4px);box-shadow:0 1px 0 #8A8A8A,0 2px 0 #5A5A5A}
.cb.dk{background:#141414;color:#F2F2F2;box-shadow:0 0 0 1.5px #3A3A3A,0 4px 0 #000}
.cb.dk .in{border-color:#5A5A5A}.cb.dk .s{color:#8C8C8C}
.cb.off{opacity:.45;pointer-events:none}
.lnk{font:inherit;background:none;border:0;color:#A3A3A3;text-decoration:underline;text-underline-offset:3px;font-size:12px;cursor:pointer;padding:6px}
.opt{font:inherit;text-align:left;cursor:pointer;color:#F2F2F2}
.idle{position:relative;overflow:hidden}
.idle .bar{position:absolute;left:0;bottom:0;height:3px;background:#F2F2F2;animation:bar linear forwards}
@keyframes bar{from{width:0}to{width:100%}}
.dz{cursor:pointer;background:none;border:0;padding:0}
.reach{border:0;padding:0;cursor:pointer}
.reach.on{background:rgba(46,230,166,.65);box-shadow:inset 0 0 0 2px #F2F2F2}
.act{font:inherit;text-align:left;cursor:pointer}
.sent{display:flex;align-items:center;gap:10px;padding:12px 14px;border-radius:12px;background:#141414;border:1.5px dashed #5E5E5E;font-size:13px;color:#D4D4D4}
.spin{width:14px;height:14px;border-radius:50%;border:2px solid #5E5E5E;border-top-color:#F2F2F2;animation:spin .8s linear infinite}
.zoom{position:absolute;inset:0;z-index:50;background:rgba(5,5,5,.92);display:flex;flex-direction:column;align-items:center;justify-content:center;gap:16px;padding:20px;cursor:pointer}
.ok{display:inline-block;font-family:'Cinzel',serif;font-weight:800;font-size:22px;color:#0A0A0A;background:#F2F2F2;padding:4px 16px;border-radius:8px;box-shadow:0 4px 0 #6E6E6E}
.slam{position:absolute;left:0;right:0;top:300px;z-index:60;display:grid;place-items:center;pointer-events:none;animation:slamOut 2.2s ease-in forwards}
.slam span{font-family:'Cinzel',serif;font-weight:800;font-size:40px;color:#0A0A0A;background:#FFD60A;padding:6px 22px;transform:rotate(-4deg);box-shadow:0 6px 0 #8A6F00,0 20px 40px rgba(0,0,0,.6);animation:slamIn 2.2s cubic-bezier(.2,1.4,.3,1) forwards}
@keyframes slamIn{0%{transform:rotate(-4deg) scale(3);opacity:0}15%{transform:rotate(-4deg) scale(.92);opacity:1}22%,100%{transform:rotate(-4deg) scale(1)}}
@keyframes slamOut{0%,70%{opacity:1}100%{opacity:0}}
.endc{position:absolute;inset:0;z-index:70;background:rgba(5,5,5,.94);display:flex;flex-direction:column;align-items:center;justify-content:center;gap:18px;text-align:center;padding:30px}
.side{display:flex;flex-direction:column;gap:14px;width:440px;flex:none}
.disc{display:flex;flex-direction:column;gap:12px;padding:18px;border-radius:16px;background:#111214;border:1px solid #2A2C30}
.disc .hdr{display:flex;align-items:center;gap:8px;font-size:12px;color:#9AA0AA;font-weight:600}
.disc .live{width:8px;height:8px;border-radius:50%;background:#2EE6A6;box-shadow:0 0 8px #2EE6A6}
.line{display:flex;gap:12px;align-items:flex-start;animation:lineIn .5s ease-out both}
@keyframes lineIn{from{opacity:0;transform:translateY(6px)}to{opacity:1;transform:none}}
.av{width:38px;height:38px;border-radius:50%;display:grid;place-items:center;font-weight:700;font-size:14px;flex:none;background:#2A2C30}
.av.mj{background:#EDEDED;color:#0A0A0A;box-shadow:0 0 0 3px #2EE6A6}
.line b{display:block;font-size:13px;margin-bottom:2px}
.line p{margin:0;font-size:15px;line-height:1.45;color:#E5E5E5}
.line.mjl p{font-family:'Cormorant Garamond',Georgia,serif;font-style:italic;font-size:19px;line-height:1.3}
.think{position:relative;padding:16px 18px;border-radius:18px;background:#EDEDED;color:#0A0A0A;font-family:'Cormorant Garamond',Georgia,serif;font-style:italic;font-size:21px;line-height:1.3;box-shadow:0 4px 0 #8A8A8A}
.think::after{content:'';position:absolute;left:-14px;top:24px;width:18px;height:18px;border-radius:50%;background:#EDEDED}
.think::before{content:'';position:absolute;left:-26px;top:40px;width:9px;height:9px;border-radius:50%;background:#EDEDED}
.prog{display:flex;flex-wrap:wrap;gap:6px}
.hand{position:relative;height:150px;margin-top:6px}
.hand .cw{position:absolute;bottom:0;background:none;border:0;padding:0;cursor:pointer;transform-origin:50% 140%;transition:transform .25s cubic-bezier(.2,1.3,.3,1),filter .25s}
.hand .cw.sel{filter:drop-shadow(0 0 14px rgba(255,255,255,.35));z-index:5}
.other{width:80px;height:112px;border-radius:9px;border:2px dashed #8C8C8C;background:#0E0E0E;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:6px;color:#F2F2F2;box-sizing:border-box;box-shadow:0 4px 0 #000}
.other b{font-family:'Cinzel',serif;font-size:30px;line-height:1}
.other span{font-family:'Cinzel',serif;font-size:12px;font-weight:800}
.cw.sel .other{border-color:#F2F2F2;background:#1A1A1A}
.txt{font:inherit;font-size:14px;width:100%;box-sizing:border-box;min-height:64px;resize:none;padding:10px 12px;border-radius:10px;background:#0A0A0A;color:#F2F2F2;border:1.5px solid #F2F2F2;outline:none}
.txt::placeholder{color:#6E6E6E}
.cardinfo{display:flex;justify-content:space-between;align-items:baseline;gap:8px;font-size:12px;color:#A3A3A3;margin-top:8px}
.cardinfo b{font-family:'Cinzel',serif;font-size:15px;color:#F2F2F2}
.mapov{position:absolute;left:0;right:0;top:96px;bottom:64px;z-index:40;background:#080808;display:flex;flex-direction:column;gap:10px;padding:12px 0 0;animation:lineIn .25s ease-out both}
.mapov .mw{position:relative;height:420px;overflow:hidden;border-top:1px solid #2C2C2C;border-bottom:1px solid #2C2C2C}
.here{position:absolute;z-index:970;transform:translate(-50%,-100%);font-size:10px;font-weight:700;letter-spacing:.1em;text-transform:uppercase;background:#FFD60A;color:#0A0A0A;padding:2px 6px;border-radius:4px;white-space:nowrap}
.nav button{cursor:pointer}
.cbody{flex:1;display:flex;flex-direction:column;gap:8px;padding:44px 12px 0;min-height:0;overflow:hidden}
.trkrow{display:flex;justify-content:space-between;align-items:baseline;padding:0 4px}
.trk{display:flex;align-items:center;gap:6px;padding:0 4px}
.trk .pf.now{border-color:#FFD60A;box-shadow:0 4px 0 #000,0 0 22px rgba(255,214,10,.45)}
.rail{flex:1;height:2px;background:repeating-linear-gradient(90deg,#3A3A3A 0 4px,transparent 4px 8px);min-width:6px}
.cmap{position:relative;height:210px;border-radius:16px;overflow:hidden;border:1px solid #2C2C2C;box-shadow:inset 0 0 40px rgba(0,0,0,.8);flex:none}
.cmap .hint{position:absolute;left:8px;bottom:8px;z-index:990;font-size:12px;font-weight:600;background:rgba(5,5,5,.85);border:1px solid #2C2C2C;border-radius:8px;padding:6px 10px}
.statrow{display:flex;align-items:center;gap:10px;padding:0 4px}
.gemw{display:inline-grid;flex:none;filter:drop-shadow(0 3px 0 rgba(0,0,0,.55))}
.gemw.beat{animation:gbeat .9s ease-in-out infinite}
@keyframes gbeat{0%,100%{transform:scale(1)}12%{transform:scale(1.2)}24%{transform:scale(.96)}36%{transform:scale(1.12)}}
.arcs{position:relative;height:112px;flex:none}
.arc{position:absolute;border-radius:50%;background:#0A0A0A;display:grid;place-items:center;box-shadow:0 0 0 3px #2C2C2C,0 6px 0 #000,inset 0 0 0 6px #161616;border:0;padding:0;cursor:pointer;font:inherit}
.arc .top{border-radius:50%;background:radial-gradient(circle at 40% 30%,#FFFFFF,#CFCFCF 70%);color:#0A0A0A;display:grid;place-items:center;transform:translateY(-6px);box-shadow:0 6px 0 #7A7A7A,0 10px 14px rgba(0,0,0,.6);transition:transform .08s,box-shadow .08s}
.arc.dk .top{background:radial-gradient(circle at 40% 30%,#3A3A3A,#161616 70%);color:#F2F2F2;box-shadow:0 6px 0 #000,0 0 0 1.5px #5A5A5A}
.arc:active .top{transform:translateY(-1px);box-shadow:0 1px 0 #7A7A7A}
.arc.big::before{content:'';position:absolute;inset:-6px;border-radius:50%;border:2px solid #F2F2F2;opacity:0;animation:halo 2.4s ease-out infinite}
@keyframes halo{0%,78%{transform:scale(1);opacity:0}80%{opacity:.9}100%{transform:scale(1.3);opacity:0}}
.arc.busy{pointer-events:none;opacity:.6}
.alab{position:absolute;width:90px;text-align:center;font-size:11px;color:#A3A3A3}
.cres{display:flex;flex-direction:column;gap:6px;align-items:center;justify-content:center;text-align:center;height:132px;flex:none}
.ov{position:absolute;left:0;right:0;top:96px;bottom:64px;z-index:40;background:#080808;overflow-y:auto;padding:14px 16px 20px;display:flex;flex-direction:column;gap:16px;animation:lineIn .25s ease-out both;box-sizing:border-box}
.sec{display:flex;flex-direction:column;gap:8px}
.trait{display:flex;align-items:center;gap:10px;padding:10px 12px;border-radius:10px;background:#141414;border:1px solid #2C2C2C;font-size:13px}
.trait span:last-child{margin-left:auto;font-size:12px;color:#8C8C8C}
.mini-hand{display:flex;gap:6px}
.slots{display:grid;grid-template-columns:repeat(5,56px);gap:8px}
.wear{display:flex;align-items:center;gap:12px;padding:8px 10px;border-radius:10px;background:#141414;border:1px solid #2C2C2C;font-size:13px}
.wear small{display:block;font-size:11px;color:#8C8C8C}
.tl{display:flex;flex-direction:column;gap:0;border-left:2px solid #2C2C2C;margin-left:6px}
.ev{position:relative;padding:0 0 14px 18px;font-size:13px;line-height:1.45;color:#D4D4D4;animation:lineIn .4s ease-out both}
.ev::before{content:'';position:absolute;left:-7px;top:4px;width:12px;height:12px;border-radius:3px;background:#3A3A3A;box-shadow:0 0 0 3px #080808}
.ev.key::before{background:#F2F2F2}
.ev time{display:block;font-size:10px;font-weight:700;letter-spacing:.14em;text-transform:uppercase;color:#6E6E6E}
.clue2{font:inherit;color:#F2F2F2;text-align:left;cursor:pointer;display:flex;align-items:center;gap:12px;font-size:13px;padding:8px;border-radius:10px;background:#141414;border:1px solid #2C2C2C}
.clue2 .new{margin-left:auto;font-size:9px;font-weight:700;letter-spacing:.12em;background:#F2F2F2;color:#0A0A0A;padding:2px 5px;border-radius:3px}
.prog button{font:inherit;font-size:12px;font-weight:700;width:34px;height:34px;border-radius:8px;border:1px solid #2C2C2C;background:#141414;color:#8C8C8C;cursor:pointer}
.prog button.done{color:#F2F2F2}
.prog button.cur{background:#EDEDED;color:#0A0A0A;border-color:#0A0A0A;box-shadow:0 2px 0 #8A8A8A}
.ctl{display:flex;gap:8px}
.ctl button{font:inherit;flex:1;min-height:42px;border-radius:10px;border:1px solid #3A3A3A;background:#141414;color:#F2F2F2;font-weight:700;font-size:13px;cursor:pointer}
'''
def cb(label, sub='', on='', dark=False, extra=''):
    s = f'<span class="s">{sub}</span>' if sub else ''
    return f'<button type="button" class="cb {"dk" if dark else ""} {extra}" onClick="{{{{{on}}}}}"><span class="in"><span style="flex: 1"><span class="t">{label}</span>{s}</span><span class="chev">›</span></span></button>'
def idle(text, ms):
    return f'<div class="idle">{text}<span class="bar" style="animation-duration: {ms}ms"></span></div>'
HEAD = '''<sc-if value="{{headOn}}" hint-placeholder-val="{{ true }}"><div class="hd"><span class="pf"><dc-import name="Sprite" perso="borin" px="1" anim="repos" hint-size="20px,26px"></dc-import></span><div style="display: flex; flex-direction: column; gap: 3px; flex: 1"><span class="ttl" style="font-size: 14px">Borin</span><dc-import name="Coeurs" pv="{{hp}}" max="31" coeurs="8" px="2" degats="{{deg}}" hint-size="151px,16px"></dc-import></div><span class="pv">{{hp}}<small> / 31 PV</small></span></div></sc-if>'''
NAV = '<nav class="nav"><sc-for list="{{tabs}}" as="t" hint-placeholder-count="4"><button type="button" class="{{t.cls}}" onClick="{{t.pick}}">{{t.n}}</button></sc-for></nav>'
def bn(kind, text): return f'<div class="bn {kind}"><span>{text}</span></div>'
def block(n, banner, body, foot):
    return f'<sc-if value="{{{{is{n}}}}}" hint-placeholder-val="{{{{ {"true" if n==1 else "false"} }}}}">{banner}<div class="body">{body}</div><div class="ft">{foot}</div></sc-if>'
B = []
B.append(block(1, bn('wait', '⏳ On attend le MJ'), '''<div style="display: flex; flex-direction: column; gap: 4px"><span class="lbl">Ce soir · 20 h 30</span><span class="ttl" style="font-size: 26px; line-height: 1.1">La crypte des Valombre</span><span style="font-size: 13px; color: #A3A3A3">Session 3</span></div>
<div class="stage" style="margin-top: 16px"><dc-import name="Perso" perso="borin" px="5" etat="aucun" hint-size="100px,130px"></dc-import><div style="display: flex; flex-direction: column; gap: 4px"><span class="lbl">Tu joues</span><span class="ttl" style="font-size: 22px">Borin</span><span style="font-size: 12px; color: #A3A3A3">nain guerrier · niveau 3</span></div></div>
<div style="display: flex; flex-direction: column; gap: 8px; margin-top: 16px"><span class="lbl">Autour de la table</span><div class="who"><span class="dot on"></span><b>Romain</b><span>le MJ</span></div><div class="who"><span class="dot on"></span><b>Camille</b><span>joue Lyra</span></div><div class="who"><span class="dot on"></span><b>Hugo</b><span>joue Sef</span></div></div>''',
  '<sc-if value="{{noSound}}" hint-placeholder-val="{{ true }}">' + cb('Activer le son', 'La musique de la partie passera ici', 'sound') + '<button type="button" class="lnk" onClick="{{mute}}">Continuer sans le son</button></sc-if><sc-if value="{{soundOn}}" hint-placeholder-val="{{ false }}"><div class="sent"><span class="spin"></span>C’est parti. Le MJ lance la partie…</div></sc-if>'))
B.append(block(2, bn('listen', '👂 Le MJ commence'), '''<span class="ttl" style="font-size: 26px">Précédemment…</span>
<p class="nar" style="font-size: 20px; margin-top: 12px">Vous êtes descendus dans la crypte des Valombre. Les crânes regardent tous la porte nord. Borin a pris un mauvais coup.</p>
<div style="display: flex; flex-direction: column; gap: 8px; margin-top: 16px"><span class="lbl">Ce que vous savez</span><div class="clue"><dc-import name="Objet" objet="parchemin" rarete="commune" taille="40" pips="false" hint-size="40px,40px"></dc-import><span><b>Le registre</b> : une page a été arrachée.</span></div><div class="clue"><dc-import name="Objet" objet="anneau" rarete="peu" taille="40" pips="false" hint-size="40px,40px"></dc-import><span><b>La bague du gardien</b> : gravée d’un corbeau.</span></div></div>''', idle('Rien à faire : écoute le MJ', 7000)))
B.append(block(3, bn('listen', '👂 Le MJ raconte · écoute'), '''<div class="art"><span class="ttl" style="font-size: 22px; color: #F2F2F2">La salle de l’autel</span><span class="tag">[ILLUSTRATION]</span></div>
<p class="nar typed" style="font-size: 20px; margin-top: 14px">L’escalier s’arrête devant un autel de pierre noire. Des cendres encore tièdes. Quelqu’un est passé ici il y a moins d’une heure.</p>
<div class="mini" style="margin-top: 14px"><span class="eq"><i></i><i></i><i></i></span><span style="flex: 1">Ombres de la crypte</span><span style="color: #8C8C8C">{{soundTxt}}</span></div>''', idle('Rien à faire pour l’instant', 8000)))
B.append(block(4, bn('you', '✋ À vous : que faites-vous ?'), '''<p class="nar" style="font-size: 19px">« Devant l’autel, que faites-vous ? »</p>
<div style="display: flex; flex-direction: column; gap: 8px; margin-top: 10px"><sc-for list="{{opts}}" as="o" hint-placeholder-count="4"><button type="button" class="opt {{o.cls}}" onClick="{{o.pick}}"><b>{{o.t}}</b><span>{{o.d}}</span></button></sc-for></div>
<sc-if value="{{freeOpen}}" hint-placeholder-val="{{ false }}"><textarea class="txt" style="margin-top: 8px" placeholder="Ex. : je verse de l’eau sur les cendres pour voir ce qu’elles cachent"></textarea></sc-if>
<div class="others" style="margin-top: 10px"><span class="lbl">Les autres</span><span>Lyra surveille l’escalier.</span></div>''',
  '<sc-if value="{{notSent}}" hint-placeholder-val="{{ true }}">' + cb('Proposer au MJ', '{{choiceName}}', 'propose') + '</sc-if><sc-if value="{{sent}}" hint-placeholder-val="{{ false }}"><div class="sent"><span class="spin"></span>Envoyé. Le MJ lit ta proposition…</div></sc-if>'))
B.append(block(5, bn('you', '🎲 À toi : lance le dé'), '''<div style="display: flex; flex-direction: column; gap: 12px; align-items: center; text-align: center">
<span class="ttl" style="font-size: 22px">{{diceTitle}}</span>
<span style="font-size: 15px; color: #D4D4D4">Il te faut <b style="font-size: 20px; color: #F2F2F2">13 ou plus</b></span>
<sc-if value="{{diceIdle}}" hint-placeholder-val="{{ true }}"><button type="button" class="dz" onClick="{{roll}}" aria-label="Lancer le dé"><dc-import name="De" de="d20" valeur="20" taille="150" anim="survol" hint-size="150px,150px"></dc-import></button><span style="font-size: 13px; color: #A3A3A3">Touche le dé</span></sc-if>
<sc-if value="{{diceRolling}}" hint-placeholder-val="{{ false }}"><dc-import name="De" de="d20" valeur="16" taille="150" anim="roule" hint-size="150px,150px"></dc-import></sc-if>
<sc-if value="{{diceDone}}" hint-placeholder-val="{{ false }}"><dc-import name="De" de="d20" valeur="16" taille="150" anim="fixe" hint-size="150px,150px"></dc-import><div class="res" style="animation: none"><span>16</span><span class="op">+ 4</span><span class="op">=</span><span>20</span></div><span class="win" style="animation: none">Réussi !</span><span style="font-size: 12px; color: #8C8C8C">+4, c’est ton bonus de Sagesse.</span></sc-if>
</div>''', '<sc-if value="{{diceIdle}}" hint-placeholder-val="{{ true }}">' + cb('Lancer le dé', 'Il te faut 13 ou plus', 'roll') + '</sc-if><sc-if value="{{diceNotIdle}}" hint-placeholder-val="{{ false }}"><div class="sent"><span class="spin"></span>Le MJ découvre ton résultat…</div></sc-if>'))
B.append(block(6, bn('star', '★ Tu as trouvé quelque chose'), '''<div style="display: flex; flex-direction: column; gap: 14px; align-items: center">
<button type="button" class="dz flip" onClick="{{zoom}}"><dc-import name="GameCard" titre="La page arrachée" type="Indice" texte="Coincée sous l’autel : la page qui manquait au registre." valeur="" stat="none" icon="parchemin" w="220" hint-size="220px,308px"></dc-import></button>
<span style="font-size: 13px; color: #A3A3A3; text-align: center">Rangée dans le journal. Tout le groupe la voit.</span></div>
<sc-if value="{{zoomed}}" hint-placeholder-val="{{ false }}"><div class="zoom" onClick="{{zoom}}"><span class="lbl">La page arrachée</span><p class="nar" style="font-size: 22px; text-align: center">« Ici reposent les Valombre, gardiens de la porte nord. Que nul ne chante leur nom après le couvre-feu. »</p><span style="font-size: 12px; color: #8C8C8C">Touche pour fermer</span></div></sc-if>''',
  cb('Lire en grand', '', 'zoom', dark=True) + idle('Le MJ continue dans un instant', 6500)))
B.append(block(7, bn('you', '✋ Exploration · déplace-toi'), '''<div class="mapw" style="margin: 0 -16px"><div style="position: absolute; left: -12px; top: -8px"><dc-import name="Plan" theme="crypte" carte="salle" pitch="28" vue="joueur" bx="{{bx}}" by="{{by}}" hint-size="392px,280px"></dc-import></div>
<sc-if value="{{notMoved}}" hint-placeholder-val="{{ true }}"><sc-for list="{{reach}}" as="r" hint-placeholder-count="10"><button type="button" class="reach {{r.cls}}" aria-label="Aller ici" style="left: {{r.l}}px; top: {{r.t}}px" onClick="{{r.pick}}"></button></sc-for></sc-if></div>
<div class="legend" style="margin-top: 10px; padding: 0"><span><i class="g"></i>tu peux y aller</span><span><i class="f"></i>pas encore vu</span></div>
<div style="font-size: 13px; color: #D4D4D4; margin-top: 8px">{{moveTxt}}</div>''',
  '<sc-if value="{{notMoved}}" hint-placeholder-val="{{ true }}">' + cb('Y aller', '{{moveSub}}', 'move', extra='{{moveCls}}') + '</sc-if><sc-if value="{{moved}}" hint-placeholder-val="{{ false }}"><div class="sent"><span class="spin"></span>Tu avances… quelque chose bouge dans l’ombre.</div></sc-if>'))
B.append(block(8, bn('wait', '⏳ Combat · {{who}} joue'), '''<div class="order">
<sc-for list="{{order}}" as="o" hint-placeholder-count="4"><div class="o {{o.cls}}"><span class="pf {{o.pf}}"><dc-import name="Sprite" perso="{{o.p}}" px="1" anim="repos" hint-size="20px,26px"></dc-import></span><span>{{o.n}}</span></div></sc-for>
</div>
<div class="feed" style="margin-top: 14px"><span class="lbl">À l’instant</span><p>{{feedTxt}}</p></div>
<div class="feed" style="margin-top: 10px"><span class="lbl">Ton tour arrive</span><p>{{feedNext}}</p></div>''', idle('Rien à faire : regarde, ton tour arrive', 6500)))
B.append('<sc-if value="{{is9}}" hint-placeholder-val="{{ false }}"><div class="cbody">' + """
<div class="trkrow"><span class="lbl">Initiative · round 3</span><span class="lbl" style="color: #FFD60A">À ton tour</span></div>
<div class="trk" aria-label="Ordre du tour">
<span class="pf"><dc-import name="Sprite" perso="lyra" px="1" anim="repos" hint-size="20px,26px"></dc-import></span><span class="rail"></span>
<span class="pf now"><dc-import name="Sprite" perso="borin" px="2" anim="repos" hint-size="40px,52px"></dc-import></span><span class="rail"></span>
<span class="pf foe"><dc-import name="Sprite" perso="gobelin" px="1" anim="repos" hint-size="20px,26px"></dc-import></span><span class="rail"></span>
<span class="pf"><dc-import name="Sprite" perso="sef" px="1" anim="repos" hint-size="20px,26px"></dc-import></span><span class="rail"></span>
<span class="pf foe"><dc-import name="Sprite" perso="gobelin" px="1" anim="repos" hint-size="20px,26px"></dc-import></span>
</div>
<div class="cmap"><div style="position: absolute; left: 18px; top: -138px"><dc-import name="Plan" theme="crypte" carte="salle" pitch="34" vue="joueur" grille="false" combat="true" hint-size="476px,340px"></dc-import></div><span class="tgt" style="left: 185px; top: 97px"></span><span class="hint">Cible : <span style="color: #FF9F1C">le gobelin</span>, juste à côté</span></div>
<sc-if value="{{notOtherSel}}" hint-placeholder-val="{{ true }}"><div class="statrow"><span class="gemw beat"><dc-import name="Gemme" stat="pv" valeur="{{hp}}" taille="40" hint-size="40px,40px"></dc-import></span><div style="flex: 1; display: flex; flex-direction: column; gap: 3px"><dc-import name="Coeurs" pv="{{hp}}" max="31" coeurs="7" px="3" hint-size="201px,24px"></dc-import><span style="font-size: 11px; color: #FF4D5E; font-weight: 600">{{hp}} / 31 PV</span></div><span class="gemw"><dc-import name="Gemme" stat="ca" valeur="18" taille="34" hint-size="34px,34px"></dc-import></span><span class="gemw"><dc-import name="Gemme" stat="mvt" valeur="6" taille="34" hint-size="34px,34px"></dc-import></span></div></sc-if>
<sc-if value="{{otherSel}}" hint-placeholder-val="{{ false }}"><textarea class="txt" placeholder="Décris ton idée : je renverse la table sur le gobelin…"></textarea></sc-if>
<sc-if value="{{atkIdle}}" hint-placeholder-val="{{ true }}"><div class="hand" style="height: 132px"><sc-for list="{{acts}}" as="a" hint-placeholder-count="4"><button type="button" class="cw {{a.cls}}" style="left: {{a.x}}px; transform: rotate({{a.r}}deg) translateY({{a.y}}px)" onClick="{{a.pick}}" aria-label="{{a.t}}"><sc-if value="{{a.isCard}}" hint-placeholder-val="{{ true }}"><dc-import name="GameCard" titre="{{a.t}}" type="{{a.ty}}" texte="{{a.tx}}" valeur="{{a.v}}" stat="{{a.st}}" icon="{{a.ic}}" rarete="{{a.ra}}" etat="{{a.et}}" w="80" hint-size="80px,112px"></dc-import></sc-if><sc-if value="{{a.isOther}}" hint-placeholder-val="{{ false }}"><span class="other"><b>?</b><span>Autre…</span></span></sc-if></button></sc-for></div></sc-if>
<sc-if value="{{atkBusy}}" hint-placeholder-val="{{ false }}"><div class="cres"><sc-if value="{{atkRolling}}" hint-placeholder-val="{{ false }}"><dc-import name="De" de="d20" valeur="17" taille="80" anim="roule" couleur="atq" hint-size="80px,80px"></dc-import></sc-if><div class="sent"><span class="spin"></span>{{busyTxt}}</div></div></sc-if>
<sc-if value="{{atkHit}}" hint-placeholder-val="{{ false }}"><div class="cres"><div class="res" style="animation: none; font-size: 28px"><span>17</span><span class="op">+ 5</span><span class="op">=</span><span>22</span></div><span class="ok">Touché !</span><span style="font-size: 14px">La hache fait <b style="color: #FF4D5E">9 dégâts</b>.</span></div></sc-if>
<sc-if value="{{atkCustom}}" hint-placeholder-val="{{ false }}"><div class="cres"><span class="ok">Le MJ accepte !</span><span style="font-size: 14px">Tu renverses la table : <b>le gobelin tombe à terre</b>.</span></div></sc-if>
<sc-if value="{{atkSelf}}" hint-placeholder-val="{{ false }}"><div class="cres"><span class="ok">{{selfTxt}}</span><span style="font-size: 14px">{{selfSub}}</span></div></sc-if>
<div class="arcs" aria-label="Commandes">
<button type="button" class="arc dk" aria-label="Ton sac" onClick="{{openBag}}" style="left: 40px; top: 26px; width: 58px; height: 58px"><span class="top" style="width: 44px; height: 44px"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10 2v7.5L4.5 19A2 2 0 0 0 6.2 22h11.6a2 2 0 0 0 1.7-3L14 9.5V2M8.5 2h7M7 16h10"></path></svg></span></button>
<span class="alab" style="left: 24px; top: 94px">Sac</span>
<button type="button" class="arc big {{bigCls}}" aria-label="{{atkLabel}}" onClick="{{bigAct}}" style="left: 131px; top: 0; width: 104px; height: 104px"><span class="top" style="width: 80px; height: 80px"><span style="display: grid; place-items: center"><svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="{{bigIcon}}"></path></svg><span class="ttl" style="font-size: 12px; margin-top: 2px">{{bigLbl}}</span></span></span></button>
<button type="button" class="arc dk" aria-label="Fin du tour" onClick="{{finish}}" style="left: 268px; top: 26px; width: 58px; height: 58px"><span class="top" style="width: 44px; height: 44px"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M5 22h14M5 2h14M17 22v-4.172a2 2 0 0 0-.586-1.414L12 12l-4.414 4.414A2 2 0 0 0 7 17.828V22M7 2v4.172a2 2 0 0 0 .586 1.414L12 12l4.414-4.414A2 2 0 0 0 17 6.172V2"></path></svg></span></button>
<span class="alab" style="left: 252px; top: 94px">Fin du tour</span>
</div>""" + '</div></sc-if>')
B.append(block(10, bn('hurt', '♥ Le gobelin te touche : −6 PV'), '''<div style="display: flex; flex-direction: column; gap: 14px; align-items: center; text-align: center">
<div class="shake"><dc-import name="Perso" perso="borin" px="5" anim="touche" etat="aucun" hint-size="100px,130px"></dc-import></div>
<span class="big">−6</span><span style="font-size: 16px">Il te reste <b>{{hp}} PV sur 31</b></span>
<sc-if value="{{danger}}" hint-placeholder-val="{{ true }}"><div class="tip"><b>Tu es en danger.</b> À ton tour, ta carte <b>Second souffle</b> te rend 1d10 + 3 PV.</div></sc-if><sc-if value="{{safe}}" hint-placeholder-val="{{ false }}"><div class="tip"><b>Tu tiens le coup</b> grâce à ton second souffle.</div></sc-if></div>''', idle('Rien à faire : c’est au tour de Lyra', 5000)))
B.append(block(11, bn('star', '★ Butin !'), '''<div style="display: flex; flex-direction: column; gap: 12px; align-items: center; text-align: center">
<div class="beam"><dc-import name="Objet" objet="hache" rarete="leg" taille="140" hint-size="140px,140px"></dc-import></div>
<span class="ttl" style="font-size: 24px; color: #F3CC63">Hache des Valombre</span><span class="lbl" style="color: #F3CC63">Légendaire</span>
<span style="font-size: 15px">Mieux que ta hache : <b>+2 pour toucher</b>.</span>
<sc-if value="{{equipped}}" hint-placeholder-val="{{ false }}"><span class="ok">{{lootTxt}}</span></sc-if></div>''',
  '<sc-if value="{{notEquipped}}" hint-placeholder-val="{{ true }}">' + cb('L’équiper', 'Remplace ta hache', 'equip') + cb('La garder dans le sac', '', 'keep', dark=True) + '</sc-if>'))
B.append(block(12, bn('listen', '★ Fin de la session'), '''<span class="ttl" style="font-size: 26px">Ce soir</span>
<ul class="rec" style="margin-top: 10px"><li>Vous avez trouvé la page arrachée du registre.</li><li>Vous avez vaincu les gobelins de la crypte.</li><li>La porte nord est ouverte. Quelque chose chante derrière.</li></ul>
<div class="gain" style="margin-top: 14px"><dc-import name="Objet" objet="hache" rarete="leg" taille="48" pips="false" hint-size="48px,48px"></dc-import><div style="display: flex; flex-direction: column; gap: 6px; flex: 1"><span style="font-size: 13px">Borin · bientôt niveau 4</span><dc-import name="Cases" stat="ini" valeur="8" max="10" h="10" largeur="200" hint-size="200px,16px"></dc-import></div></div>
<div class="next" style="margin-top: 14px"><span class="lbl">Prochaine session</span><span class="ttl" style="font-size: 20px">Jeudi 10 octobre · 20 h 30</span></div>''', cb('À jeudi !', 'Ajouter à mon agenda', 'end')))
MAPOV = '''<sc-if value="{{showMap}}" hint-placeholder-val="{{ false }}"><div class="mapov"><div style="display: flex; justify-content: space-between; align-items: baseline; padding: 0 16px"><span class="ttl" style="font-size: 18px">La crypte</span><span class="lbl">ce que le groupe a vu</span></div>
<div class="mw"><div style="position: absolute; left: -26px; top: 40px"><dc-import name="Plan" theme="crypte" carte="salle" pitch="30" vue="joueur" bx="{{bx}}" by="{{by}}" hint-size="420px,300px"></dc-import></div><span class="here" style="left: {{hereX}}px; top: {{hereY}}px">Tu es ici</span></div>
<div class="legend" style="padding: 0 16px"><span><i class="f"></i>pas encore vu</span><span>Glisse pour te déplacer sur la carte</span></div>
<div style="padding: 0 14px">''' + cb('Retour au jeu', '', 'backToGame', dark=True) + '''</div></div></sc-if>'''
PERSO = '''<sc-if value="{{showPerso}}" hint-placeholder-val="{{ false }}"><div class="ov">
<div class="stage"><dc-import name="Perso" perso="borin" px="4" etat="aucun" hint-size="80px,104px"></dc-import><div style="display: flex; flex-direction: column; gap: 4px; flex: 1"><span class="ttl" style="font-size: 22px">Borin</span><span style="font-size: 12px; color: #A3A3A3">Nain guerrier · niveau 3</span><dc-import name="Cases" stat="ini" valeur="{{xp}}" max="10" h="8" largeur="170" hint-size="170px,14px"></dc-import><span style="font-size: 11px; color: #8C8C8C">{{xpTxt}}</span></div></div>
<div class="sec"><span class="lbl">Ta vie</span><div class="trait"><dc-import name="Coeurs" pv="{{hp}}" max="31" coeurs="8" px="2" anim="false" hint-size="151px,16px"></dc-import><span>{{hp}} / 31 PV</span></div></div>
<div class="sec"><span class="lbl">Ce que tu es</span>
<div class="trait"><dc-import name="Gemme" stat="atq" valeur="+5" taille="30" anim="false" hint-size="30px,30px"></dc-import><b>Fort</b><span>tu touches souvent</span></div>
<div class="trait"><dc-import name="Gemme" stat="ca" valeur="18" taille="30" anim="false" hint-size="30px,30px"></dc-import><b>Solide</b><span>on te touche sur 18 ou plus</span></div>
<div class="trait"><dc-import name="Gemme" stat="mvt" valeur="6" taille="30" anim="false" hint-size="30px,30px"></dc-import><b>Lent</b><span>6 cases par tour</span></div>
<div class="trait"><dc-import name="Gemme" stat="ini" valeur="+1" taille="30" anim="false" hint-size="30px,30px"></dc-import><b>Attentif</b><span>bon pour fouiller (+4)</span></div></div>
<div class="sec"><span class="lbl">Ce que tu sais faire</span><div class="mini-hand">
<dc-import name="GameCard" titre="Hache d’armes" type="Arme" texte="1d12 + 3 dégâts." valeur="+5" stat="atq" icon="epee" rarete="commune" w="80" hint-size="80px,112px"></dc-import>
<dc-import name="GameCard" titre="Second souffle" type="Compétence" texte="Soigne 1d10 + 3." valeur="d10" stat="pv" icon="sort" rarete="peu-commune" w="80" hint-size="80px,112px"></dc-import>
<dc-import name="GameCard" titre="Rage du nain" type="Compétence" texte="+2 dégâts." valeur="+2" stat="atq" icon="epee" rarete="rare" w="80" hint-size="80px,112px"></dc-import>
<dc-import name="GameCard" titre="Fouiller" type="Sagesse" texte="Chercher, observer." valeur="+4" stat="none" icon="oeil" rarete="commune" w="80" hint-size="80px,112px"></dc-import></div></div>
<div class="sec"><span class="lbl">Sur toi</span>
<div class="wear"><dc-import name="Objet" objet="hache" rarete="{{weaponRar}}" taille="44" pips="false" hint-size="44px,44px"></dc-import><div><b>{{weaponName}}</b><small>arme en main</small></div></div>
<div class="wear"><dc-import name="Objet" objet="bouclier" rarete="rare" taille="44" pips="false" hint-size="44px,44px"></dc-import><div><b>Écu de fer</b><small>au bras · compte dans « Solide »</small></div></div></div>
<div class="sec"><span class="lbl">Ton sac</span><div class="slots"><sc-for list="{{bag}}" as="b" hint-placeholder-count="10"><dc-import name="Objet" objet="{{b.o}}" rarete="{{b.r}}" taille="56" qte="{{b.q}}" pips="false" hint-size="56px,56px"></dc-import></sc-for></div></div>
</div></sc-if>'''
JOURNAL = '''<sc-if value="{{showJournal}}" hint-placeholder-val="{{ false }}"><div class="ov">
<div class="sec"><span class="lbl">Les indices du groupe</span><sc-for list="{{clues}}" as="c" hint-placeholder-count="3"><button type="button" class="clue2" onClick="{{c.pick}}"><dc-import name="Objet" objet="{{c.o}}" rarete="{{c.r}}" taille="40" pips="false" hint-size="40px,40px"></dc-import><span><b>{{c.t}}</b> : {{c.d}}</span><sc-if value="{{c.isNew}}" hint-placeholder-val="{{ false }}"><span class="new">NOUVEAU</span></sc-if></button></sc-for></div>
<div class="sec"><span class="lbl">Ce soir</span><div class="tl"><sc-for list="{{events}}" as="e" hint-placeholder-count="4"><div class="ev {{e.cls}}"><time>{{e.h}}</time>{{e.t}}</div></sc-for></div></div>
<sc-if value="{{clueOpen}}" hint-placeholder-val="{{ false }}"><div class="zoom" onClick="{{closeClue}}"><span class="lbl">{{clueTitle}}</span><p class="nar" style="font-size: 22px; text-align: center">{{clueText}}</p><span style="font-size: 12px; color: #8C8C8C">Touche pour fermer</span></div></sc-if>
</div></sc-if>'''
phone = '<div class="ph">' + HEAD + ''.join(B) + MAPOV + PERSO + JOURNAL + NAV + '''<sc-if value="{{slam}}" hint-placeholder-val="{{ false }}"><div class="slam"><span>À toi, Borin !</span></div></sc-if>
<sc-if value="{{ended}}" hint-placeholder-val="{{ false }}"><div class="endc"><span class="ttl" style="font-size: 30px">Bonne nuit, Marc.</span><p class="nar" style="font-size: 20px">La porte nord vous attend jeudi.</p><button type="button" class="cb" style="width: 240px" onClick="{{restart}}"><span class="in"><span style="flex: 1"><span class="t">Rejouer la soirée</span></span><span class="chev">›</span></span></button></div></sc-if></div>'''
side_l = '''<div class="side"><div class="disc"><div class="hdr"><span class="live"></span>Discord · salon vocal « Crypte »</div>
<sc-for list="{{lines}}" as="l" hint-placeholder-count="2"><div class="line {{l.cls}}"><span class="av {{l.av}}">{{l.ini}}</span><div><b>{{l.who}}</b><p>{{l.txt}}</p></div></div></sc-for></div>
<p style="margin: 0; font-size: 12px; color: #6E6E6E; line-height: 1.5">À gauche, ce que Marc entend dans son casque. Au centre, son téléphone : touche les boutons comme lui. Quand il n’a rien à faire, le MJ continue tout seul.</p></div>'''
side_r = '''<div class="side" style="width: 380px"><span class="lbl">Dans la tête de Marc</span><div class="think">{{thought}}</div>
<div style="display: flex; flex-direction: column; gap: 10px; margin-top: 20px"><span class="lbl">La soirée · moment {{m}} sur 12</span><div class="prog"><sc-for list="{{dots}}" as="d" hint-placeholder-count="12"><button type="button" class="{{d.cls}}" onClick="{{d.pick}}">{{d.n}}</button></sc-for></div>
<div class="ctl"><button type="button" onClick="{{next}}">Le MJ continue ▶</button><button type="button" onClick="{{restart}}">Recommencer</button></div></div></div>'''
JS = r'''
class Component extends DCLogic {
  constructor(props) {
    super(props);
    this.state = this.fresh();
    const pm = Number(props && props.moment);
    if (pm >= 1 && pm <= 12) { this.state.m = pm; if (pm >= 10) this.state.hp = 9; }
    const tab = props && props.onglet;
    if (['jeu', 'carte', 'perso', 'journal'].includes(tab)) this.state.tab = tab;
    this.timers = [];
    this.ac = null;
  }
  fresh() {
    return { m: 1, tab: 'jeu', clueOpen: '', clueFull: '', sound: false, soundAsked: false, choice: 0, sent: false, dice: 'idle', zoomed: false, dest: -1, moved: false, feed: 0, card: 0, atk: 'idle', equipped: '', ended: false, hp: 15, deg: 0, slam: false };
  }
  later(fn, ms) { this.timers.push(setTimeout(fn, ms)); }
  clear() { this.timers.forEach((t) => clearTimeout(t)); this.timers = []; }
  go(m) {
    if (m > 12) return;
    this.clear();
    const patch = { m, deg: 0, zoomed: false, slam: false, tab: 'jeu' };
    if (m < 10) patch.hp = 15;
    if (m === 10) { patch.hp = Math.max(1, (this.state.hp || 15) - 6); patch.deg = 6; }
    if (m <= 4) { patch.sent = false; }
    if (m <= 5) patch.dice = 'idle';
    if (m <= 7) { patch.moved = false; patch.dest = -1; }
    if (m <= 8) patch.feed = 0;
    if (m <= 9) patch.atk = 'idle';
    if (m <= 11) patch.equipped = '';
    if (m === 1) { patch.soundAsked = false; }
    this.setState(patch);
    const auto = { 2: 7000, 3: 8000, 6: 6500, 10: 5000 };
    if (auto[m]) this.later(() => this.go(m + 1), auto[m]);
    if (m === 8) {
      this.sfx('drum');
      this.later(() => { this.setState({ feed: 1 }); this.sfx('miss'); }, 2200);
      this.later(() => { this.setState({ feed: 2 }); this.sfx('hit'); }, 4400);
      this.later(() => this.go(9), 6500);
    }
    if (m === 9) { this.setState({ slam: true }); this.sfx('turn'); this.later(() => this.setState({ slam: false }), 2300); }
    if (m === 10) this.sfx('hurt');
    if (m === 11) this.sfx('loot');
    if (m === 6) this.sfx('clue');
    if (m === 12) this.sfx('win');
  }
  audio() {
    if (this.ac) return;
    try {
      const AC = window.AudioContext || window.webkitAudioContext;
      this.ac = new AC();
      const g = this.ac.createGain(); g.gain.value = 0.05; g.connect(this.ac.destination);
      [55, 82.41, 110, 164.8].forEach((f, i) => {
        const o = this.ac.createOscillator(); o.type = i > 1 ? 'triangle' : 'sine'; o.frequency.value = f;
        const og = this.ac.createGain(); og.gain.value = i > 1 ? 0.12 : 0.5;
        const lfo = this.ac.createOscillator(); lfo.frequency.value = 0.08 + i * 0.05;
        const lg = this.ac.createGain(); lg.gain.value = i > 1 ? 0.12 : 0.3;
        lfo.connect(lg); lg.connect(og.gain); o.connect(og); og.connect(g); o.start(); lfo.start();
      });
    } catch (e) { this.ac = null; }
  }
  tone(f, t0, dur, type, vol) {
    const ac = this.ac; const o = ac.createOscillator(); o.type = type; o.frequency.value = f;
    const g = ac.createGain(); const t = ac.currentTime + t0;
    g.gain.setValueAtTime(0.0001, t); g.gain.exponentialRampToValueAtTime(vol, t + 0.01); g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
    o.connect(g); g.connect(ac.destination); o.start(t); o.stop(t + dur + 0.05);
  }
  noise(t0, dur, vol, freq) {
    const ac = this.ac; const n = Math.floor(ac.sampleRate * dur);
    const buf = ac.createBuffer(1, n, ac.sampleRate); const d = buf.getChannelData(0);
    for (let i = 0; i < n; i++) d[i] = (Math.random() * 2 - 1) * (1 - i / n);
    const s = ac.createBufferSource(); s.buffer = buf;
    const f = ac.createBiquadFilter(); f.type = 'bandpass'; f.frequency.value = freq || 1800;
    const g = ac.createGain(); g.gain.value = vol;
    s.connect(f); f.connect(g); g.connect(ac.destination); s.start(ac.currentTime + t0);
  }
  sfx(k) {
    if (!this.ac) return;
    try {
      if (k === 'click') this.tone(660, 0, 0.08, 'square', 0.04);
      if (k === 'dice') for (let i = 0; i < 10; i++) this.noise(i * 0.1 + Math.random() * 0.03, 0.05, 0.5, 2400);
      if (k === 'win') [523, 659, 784, 1047].forEach((f, i) => this.tone(f, i * 0.09, 0.4, 'triangle', 0.12));
      if (k === 'hit') { this.tone(110, 0, 0.3, 'sine', 0.5); this.noise(0, 0.12, 0.6, 900); }
      if (k === 'hurt') { this.tone(80, 0, 0.5, 'sine', 0.7); this.noise(0, 0.2, 0.7, 600); this.tone(220, 0.05, 0.25, 'sawtooth', 0.05); }
      if (k === 'miss') this.noise(0, 0.25, 0.3, 3500);
      if (k === 'turn') { this.tone(392, 0, 0.15, 'square', 0.07); this.tone(587, 0.15, 0.35, 'square', 0.07); }
      if (k === 'drum') for (let i = 0; i < 4; i++) this.tone(70, i * 0.35, 0.25, 'sine', 0.6);
      if (k === 'loot') [784, 988, 1175, 1568].forEach((f, i) => this.tone(f, i * 0.07, 0.6, 'sine', 0.1));
      if (k === 'clue') [440, 554, 659].forEach((f, i) => this.tone(f, i * 0.12, 0.6, 'sine', 0.08));
    } catch (e) {}
  }
  stopAudio() { try { if (this.ac) this.ac.close(); } catch (e) {} this.ac = null; }
  renderVals() {
    const s = this.state;
    const m = s.m;
    const optsData = [
      ['Fouiller l’autel', 'Chercher ce qui est caché. Tu es bon pour ça.'],
      ['Lire les inscriptions', 'Comprendre ce qui est écrit.'],
      ['Parler aux autres', 'Proposer un plan au groupe.'],
      ['Autre chose…', 'Écris ton idée, le MJ décide.']
    ];
    const reachCells = [[3, 6], [5, 6], [6, 6], [7, 6], [8, 6], [3, 7], [4, 7], [5, 7], [6, 7], [7, 7], [3, 5]];
    const actsData = [
      { t: 'Hache d’armes', ty: 'Arme', tx: '1d12 + 3 dégâts.', v: '+5', st: 'atq', ic: 'epee', ra: 'commune', btn: 'Attaquer le gobelin', sub: 'Touche sur 13 ou plus · 1d12 + 3', hint: 'touche sur 13+' },
      { t: 'Second souffle', ty: 'Compétence', tx: 'Soigne 1d10 + 3. Une fois.', v: 'd10', st: 'pv', ic: 'sort', ra: 'peu-commune', btn: 'Reprendre mon souffle', sub: 'Soigne 1d10 + 3 PV', hint: 'une fois par combat' },
      { t: 'Rage du nain', ty: 'Compétence', tx: '+2 dégâts ce combat.', v: '+2', st: 'atq', ic: 'epee', ra: 'rare', btn: 'Entrer en rage', sub: '+2 dégâts jusqu’à la fin du combat', hint: 'rare · une fois par jour' },
      { t: 'Autre…', other: true, btn: 'Proposer au MJ', sub: 'Ton idée, le MJ décide', hint: 'le MJ décide' }
    ];
    const dest = s.dest >= 0 ? reachCells[s.dest] : null;
    const L = (who, txt, mj) => ({ who, txt, ini: who[0], cls: mj ? 'mjl' : '', av: mj ? 'mj' : '' });
    const lines = {
      1: [L('Romain · MJ', 'Salut tout le monde ! Je lance dans une minute. Activez le son sur vos téléphones.', 1), L('Camille', 'Lyra est prête !')],
      2: [L('Romain · MJ', 'Précédemment… vous êtes descendus dans la crypte des Valombre. Borin, tu as pris un mauvais coup la dernière fois.', 1)],
      3: [L('Romain · MJ', 'L’escalier s’arrête devant un autel de pierre noire. Les cendres sont encore tièdes…', 1), L('Hugo', 'Ça sent le piège, ça.')],
      4: [L('Romain · MJ', 'Devant l’autel… que faites-vous ?', 1), L('Camille', 'Lyra surveille l’escalier, arc en main.')],
      5: [L('Romain · MJ', s.choice === 3 ? 'Ah, bonne idée ! Fais-moi un jet, il te faut 13.' : 'Borin fouille ? Fais-moi un jet. Il te faut 13.', 1)],
      6: [L('Romain · MJ', 'Sous l’autel, coincée dans une fente… une page de parchemin. La page arrachée du registre !', 1), L('Hugo', 'Je le savais !')],
      7: [L('Romain · MJ', 'La porte de droite est entrouverte. Borin, tu passes devant ?', 1)],
      8: [L('Romain · MJ', 'Des gobelins surgissent ! Le premier tire sur Lyra…', 1), L('Camille', 'Noooon !')],
      9: s.atk === 'done' && s.card === 3 ? [L('Romain · MJ', 'Hmm… j’adore. La table bascule, le gobelin s’étale par terre !', 1), L('Hugo', 'Ahah, magnifique.')] : [L('Romain · MJ', 'Borin, c’est à toi. Le gobelin est juste devant toi.', 1)],
      10: [L('Romain · MJ', 'Le gobelin riposte… sa lame passe sous ton bouclier. Six dégâts.', 1), L('Hugo', 'Tiens bon, Borin !')],
      11: [L('Romain · MJ', 'Le dernier gobelin s’effondre. Dans son sac… une hache magnifique.', 1)],
      12: [L('Romain · MJ', 'On s’arrête là pour ce soir. Bravo tout le monde, à jeudi !', 1), L('Camille', 'Trop bien, à jeudi !')]
    }[m];
    const thoughts = {
      1: 'Je suis au bon endroit, je joue Borin. On attend le MJ. Ah, il faut que j’active le son.',
      2: 'Ah oui, la porte nord et le registre. Et Borin est blessé.',
      3: 'J’écoute. Ce n’est pas encore à moi.',
      4: s.choice === 3 ? 'J’ai mon idée à moi : je l’écris, le MJ décidera.' : 'Je peux fouiller, lire, parler… ou proposer autre chose.',
      5: s.dice === 'done' ? '20 ! C’est réussi, je l’ai vu tout de suite.' : 'Il me faut 13. Allez…',
      6: 'C’est important, c’est gardé. Je peux le relire quand je veux.',
      7: s.dest >= 0 ? 'Je vais là. Le gris, on ne l’a pas encore vu.' : 'Je peux aller jusqu’aux cases vertes.',
      8: 'Le gobelin attaque Lyra. Moi, c’est après Sef. Je me prépare.',
      9: s.atk === 'done' ? 'C’est fait. Plus rien à faire, je finis mon tour.' : s.card === 3 ? 'Et si je renversais la table sur lui ? Je propose au MJ.' : 'Je suis à côté du gobelin. Je choisis une carte, et j’y vais.',
      10: s.hp <= 10 ? 'Aïe. Il me reste ' + s.hp + ' PV. Au prochain tour, je me soigne.' : 'Aïe. Heureusement que j’ai repris mon souffle.',
      11: 'Une hache légendaire ! Je la prends tout de suite.',
      12: 'C’était bien. Vivement jeudi.'
    }[m];
    const feedTxt = ['Le gobelin tire sur Lyra…', 'Le gobelin tire sur Lyra… raté !', 'Sef lance une dague : touché, 4 dégâts.'][s.feed];
    const order = [
      { p: 'gobelin', n: s.feed < 2 ? 'maintenant' : 'a joué', pf: 'foe', cls: s.feed < 2 ? 'now' : '' },
      { p: 'sef', n: s.feed === 2 ? 'maintenant' : 'Sef', pf: '', cls: s.feed === 2 ? 'now' : '' },
      { p: 'borin', n: 'toi', pf: '', cls: 'me' },
      { p: 'lyra', n: 'Lyra', pf: '', cls: '' }
    ];
    const v = {
      m, hp: s.hp, deg: s.deg, soundOn: s.soundAsked, noSound: !s.soundAsked, soundTxt: s.sound ? 'son activé' : 'son coupé',
      lines, thought: thoughts,
      opts: optsData.map(([t, d], i) => ({ t, d, cls: (i === s.choice ? 'sel' : '') + (i === 3 ? ' free' : ''), pick: () => { if (!s.sent) { this.setState({ choice: i }); this.sfx('click'); } } })),
      choiceName: optsData[s.choice][0], sent: s.sent, notSent: !s.sent,
      diceIdle: s.dice === 'idle', diceRolling: s.dice === 'rolling', diceDone: s.dice === 'done', diceNotIdle: s.dice !== 'idle',
      zoomed: s.zoomed,
      bx: dest && s.moved ? dest[0] : 4, by: dest && s.moved ? dest[1] : 6,
      reach: reachCells.map(([x, y], i) => ({ l: x * 28 - 12, t: (y + 1) * 28 - 8, cls: i === s.dest ? 'on' : '', pick: () => { this.setState({ dest: i }); this.sfx('click'); } })),
      notMoved: !s.moved, moved: s.moved,
      moveTxt: dest ? 'Tu vas sur la case choisie.' : 'Touche une case verte.',
      moveSub: dest ? 'Case choisie' : 'Choisis d’abord une case', moveCls: dest ? '' : 'off',
      who: s.feed < 2 ? 'le gobelin' : 'Sef', feedTxt, feedNext: s.feed < 2 ? 'Après Sef. Tu pourras bouger de 6 cases puis attaquer.' : 'C’est bientôt à toi !', order,
      s2: s.atk === 'done' ? 'done' : 'cur', s3: s.atk === 'done' ? 'cur' : '',
      atkIdle: s.atk === 'idle', atkRolling: s.atk === 'rolling', atkDone: s.atk === 'done',
      acts: actsData.map((a, i) => ({ ...a, tx: a.tx || '', ty: a.ty || '', v: a.v || '', st: a.st || 'none', ic: a.ic || 'epee', ra: a.ra || 'aucune',
        isCard: !a.other, isOther: !!a.other, et: i === s.card ? 'actif' : 'normal', cls: i === s.card ? 'sel' : '',
        x: 10 + i * 84, r: [-9, -3, 3, 9][i], y: (i === 0 || i === 3 ? 10 : 0) - (i === s.card ? 18 : 0),
        pick: () => { this.setState({ card: i }); this.sfx('click'); } })),
      atkLabel: actsData[s.card].btn, atkSub: actsData[s.card].sub, cardHint: actsData[s.card].hint, otherSel: s.card === 3,
      atkBusy: s.atk === 'rolling' || s.atk === 'asking', busyTxt: s.atk === 'asking' ? 'Le MJ lit ton idée…' : 'Le dé roule…',
      atkAsking: s.atk === 'asking', atkHit: s.atk === 'done' && s.card === 0, atkCustom: s.atk === 'done' && s.card === 3,
      atkSelf: s.atk === 'done' && (s.card === 1 || s.card === 2),
      selfTxt: s.card === 1 ? 'Souffle repris !' : 'En rage !', selfSub: s.card === 1 ? 'Tu regagnes 7 PV.' : '+2 dégâts jusqu’à la fin du combat.',
      notOtherSel: !(s.card === 3 && s.atk === 'idle'), headOn: m !== 9,
      bigLbl: s.atk === 'done' ? 'Finir' : s.atk === 'idle' ? ['Frappe', 'Souffle', 'Rage', 'Proposer'][s.card] : '…',
      bigIcon: s.atk === 'done' ? 'M5 12h14M13 6l6 6-6 6' : s.card === 3 ? 'M22 2 11 13M22 2l-7 20-4-9-9-4z' : s.card === 1 ? 'M12 21s-8-5.2-8-11a5 5 0 0 1 8-4 5 5 0 0 1 8 4c0 5.8-8 11-8 11z' : 'M14.5 17.5 3 6V3h3l11.5 11.5M13 19l6-6M16 16l4 4M19 21l2-2',
      bigCls: s.atk === 'rolling' || s.atk === 'asking' ? 'busy' : '',
      bigAct: () => { if (this.state.atk === 'done') { this.sfx('click'); this.go(10); } else v.attack(); },
      openBag: () => { this.setState({ tab: 'perso' }); this.sfx('click'); },
      freeOpen: s.choice === 3 && !s.sent, diceTitle: s.choice === 3 ? 'Ton idée' : optsData[s.choice][0],
      showMap: s.tab === 'carte' && !s.ended, hereX: (dest && s.moved ? dest[0] : 4) * 30 - 26 + 15, hereY: ((dest && s.moved ? dest[1] : 6) + 1) * 30 + 40 - 40,
      tabs: [['jeu', 'Jeu'], ['carte', 'Carte'], ['perso', 'Personnage'], ['journal', 'Journal']].map(([id, n]) => ({ n, cls: (s.tab || 'jeu') === id ? 'on' : '', pick: () => { this.setState({ tab: id }); this.sfx('click'); } })),
      showPerso: s.tab === 'perso' && !s.ended, showJournal: s.tab === 'journal' && !s.ended,
      xp: m >= 12 ? 8 : m >= 10 ? 7 : 6, xpTxt: (m >= 12 ? 8 : m >= 10 ? 7 : 6) + ' / 10 · bientôt niveau 4',
      weaponRar: s.equipped === 'equip' ? 'leg' : 'commune', weaponName: s.equipped === 'equip' ? 'Hache des Valombre' : 'Hache d’armes',
      bag: [{ o: 'potion', r: 'commune', q: '2' }, { o: 'bourse', r: 'commune', q: '48' }, s.equipped === 'keep' ? { o: 'hache', r: 'leg', q: '' } : s.equipped === 'equip' ? { o: 'hache', r: 'commune', q: '' } : { o: 'aucun', r: 'commune', q: '' }, { o: 'aucun', r: 'commune', q: '' }, { o: 'aucun', r: 'commune', q: '' }, { o: 'aucun', r: 'commune', q: '' }, { o: 'aucun', r: 'commune', q: '' }, { o: 'aucun', r: 'commune', q: '' }, { o: 'aucun', r: 'commune', q: '' }, { o: 'aucun', r: 'commune', q: '' }],
      clues: [
        { o: 'parchemin', r: 'commune', t: 'Le registre', d: 'une page a été arrachée.', full: '« Registre des sépultures des Valombre. » Une page manque, arrachée net.', show: true, isNew: false },
        { o: 'anneau', r: 'peu', t: 'La bague du gardien', d: 'gravée d’un corbeau.', full: 'Un anneau de fer noirci, gravé d’un corbeau aux ailes ouvertes.', show: true, isNew: false },
        { o: 'parchemin', r: 'rare', t: 'La page arrachée', d: 'trouvée sous l’autel.', full: '« Ici reposent les Valombre, gardiens de la porte nord. Que nul ne chante leur nom après le couvre-feu. »', show: m >= 6, isNew: m >= 6 && m <= 8 }
      ].filter((c) => c.show).map((c) => ({ ...c, pick: () => { this.setState({ clueOpen: c.t, clueFull: c.full }); this.sfx('click'); } })),
      clueOpen: !!s.clueOpen, clueTitle: s.clueOpen || '', clueText: s.clueFull || '',
      closeClue: () => this.setState({ clueOpen: '' }),
      events: [
        [2, '20:31', 'Précédemment : la crypte, la porte nord, Borin blessé.', ''],
        [3, '20:34', 'Le groupe arrive dans la salle de l’autel.', ''],
        [5, '20:38', s.choice === 3 ? 'Borin tente son idée : réussi (20).' : 'Borin fouille l’autel : réussi (20).', ''],
        [6, '20:39', 'Indice trouvé : la page arrachée.', 'key'],
        [7, '20:42', 'Le groupe avance vers la porte de droite.', ''],
        [8, '20:43', 'Combat ! Des gobelins surgissent.', 'key'],
        [9, '20:44', s.card === 3 ? 'Borin renverse la table : le gobelin tombe.' : s.card === 1 ? 'Borin reprend son souffle.' : s.card === 2 ? 'Borin entre en rage.' : 'Borin touche le gobelin : 9 dégâts.', ''],
        [10, '20:45', 'Le gobelin blesse Borin : −6 PV.', ''],
        [11, '20:52', 'Butin : Hache des Valombre (légendaire).', 'key'],
        [12, '21:40', 'Fin de la session. Prochaine : jeudi 20 h 30.', 'key']
      ].filter((e) => e[0] <= m && !(e[0] === 9 && m === 9 && s.atk !== 'done')).reverse().map(([, h, t, cls]) => ({ h, t, cls })),
      backToGame: () => { this.setState({ tab: 'jeu' }); this.sfx('click'); },
      equipped: !!s.equipped, notEquipped: !s.equipped, lootTxt: s.equipped === 'equip' ? 'Équipée !' : 'Dans ton sac',
      slam: s.slam, ended: s.ended, danger: s.hp <= 10, safe: s.hp > 10,
      dots: Array.from({ length: 12 }, (_, i) => ({ n: i + 1, cls: i + 1 === m ? 'cur' : i + 1 < m ? 'done' : '', pick: () => this.go(i + 1) })),
      sound: () => { this.audio(); this.setState({ sound: true, soundAsked: true }); this.sfx('click'); this.later(() => this.go(2), 1800); },
      mute: () => { this.setState({ soundAsked: true }); this.later(() => this.go(2), 1200); },
      propose: () => { if (s.sent) return; this.setState({ sent: true }); this.sfx('click'); this.later(() => this.go(5), 2200); },
      roll: () => { if (this.state.dice !== 'idle') return; this.setState({ dice: 'rolling' }); this.sfx('dice'); this.later(() => { this.setState({ dice: 'done' }); this.sfx('win'); }, 1600); this.later(() => this.go(6), 4800); },
      zoom: () => { this.setState({ zoomed: !this.state.zoomed }); this.sfx('click'); },
      move: () => { if (this.state.dest < 0) return; this.setState({ moved: true }); this.sfx('click'); this.later(() => this.go(8), 2600); },
      attack: () => {
        if (this.state.atk !== 'idle') return;
        if (this.state.card === 1) { this.setState({ hp: 22, deg: 0, atk: 'done' }); this.sfx('win'); return; }
        if (this.state.card === 2) { this.setState({ atk: 'done' }); this.sfx('turn'); return; }
        if (this.state.card === 3) { this.setState({ atk: 'asking' }); this.sfx('click'); this.later(() => { this.setState({ atk: 'done' }); this.sfx('win'); }, 2600); return; }
        this.setState({ atk: 'rolling' }); this.sfx('dice');
        this.later(() => { this.setState({ atk: 'done' }); this.sfx('hit'); }, 1500);
      },
      finish: () => { this.sfx('click'); this.go(10); },
      equip: () => { this.setState({ equipped: 'equip' }); this.sfx('loot'); this.later(() => this.go(12), 1800); },
      keep: () => { this.setState({ equipped: 'keep' }); this.sfx('click'); this.later(() => this.go(12), 1800); },
      end: () => { this.clear(); this.setState({ ended: true }); this.sfx('win'); this.later(() => this.stopAudio(), 1500); },
      restart: () => { this.clear(); this.stopAudio(); this.setState(this.fresh()); },
      next: () => { if (this.state.m < 12) this.go(this.state.m + 1); }
    };
    for (let i = 1; i <= 12; i++) v['is' + i] = m === i && !s.ended;
    const seul = String(this.props.seul ?? false) === 'true';
    v.seul = seul; v.full = !seul;
    v.rootStyle = seul ? 'width: 390px; height: 844px; padding: 0' : 'width: 1440px; height: 1000px; padding: 56px 64px';
    return v;
  }
}
'''
html = f'''<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<title>Jouer la soirée de Marc</title>
<script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Cinzel:wght@600;700;800&amp;family=Chakra+Petch:wght@400;500;600;700&amp;family=Cormorant+Garamond:ital,wght@1,500;1,600&amp;display=swap">
<style>{css}</style>
</helmet>
<div class="pm" style="{{{{rootStyle}}}}; position: relative; overflow: hidden; display: flex; gap: 48px; box-sizing: border-box; align-items: flex-start">
<sc-if value="{{{{full}}}}" hint-placeholder-val="{{{{ true }}}}">{side_l}</sc-if>
{phone}
<sc-if value="{{{{full}}}}" hint-placeholder-val="{{{{ true }}}}">{side_r}</sc-if>
</div>
</x-dc>
<script type="text/x-dc" data-dc-script data-props='{{"moment":{{"editor":"int","default":1,"min":1,"max":12}},"seul":{{"editor":"boolean","default":false}},"onglet":{{"editor":"enum","options":["jeu","carte","perso","journal"],"default":"jeu"}},"$preview":{{"width":1440,"height":1000}}}}'>{JS}</script>
</body>
</html>
'''
open('design/project/Jouer-Marc.dc.html', 'w').write(html)
d = json.load(open('design/project/canvas.json'))
d['boards']['Jouer-Marc.dc.html'] = {"x": 4080, "y": 4720, "w": 1440, "h": 1000, "title": "Jouer · la soirée de Marc", "is_interactive": True}
if 'Jouer-Marc.dc.html' not in d['order']: d['order'].append('Jouer-Marc.dc.html')
json.dump(d, open('design/project/canvas.json', 'w'), ensure_ascii=False, indent=2)
print(len(html))
