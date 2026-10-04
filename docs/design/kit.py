"""Shared building blocks for the playable journey boards and their storyboards.

A playable board is a laptop, a phone, a TV or a tablet (or several side by side) frozen on one
moment of a journey, with the side panel (who thinks what, why this screen, moment picker).
`board()` writes it, `storyboard()` writes the grid of frozen frames that goes with it.
Run the board scripts from the repository root.
"""
import json, sys

sys.path.insert(0, 'docs/design')
from scope import scope, check

FONTS = '<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Cinzel:wght@600;700;800&amp;family=Chakra+Petch:wght@400;500;600;700&amp;family=Cormorant+Garamond:ital,wght@1,500;1,600&amp;display=swap">'

CSS = r'''
body{margin:0;background:#0A0A0A;overflow:hidden}
.pm{font-family:'Chakra Petch',system-ui,sans-serif;color:#F2F2F2;background:radial-gradient(120% 60% at 50% 0%,#1C1C1C 0,#0C0C0C 55%,#050505 100%);-webkit-font-smoothing:antialiased}
.ttl{font-family:'Cinzel',Georgia,serif;font-weight:800;letter-spacing:.02em}
.nar{margin:0;font-family:'Cormorant Garamond',Georgia,serif;font-style:italic;font-weight:500;line-height:1.35;color:#E5E5E5}
.lbl{font-size:10px;font-weight:600;letter-spacing:.2em;text-transform:uppercase;color:#8C8C8C}
.mut{font-size:12px;color:#8C8C8C;line-height:1.5}
.lap{position:relative;width:1440px;height:900px;flex:none;overflow:hidden;box-sizing:border-box;display:grid;grid-template-rows:52px 1fr 78px;background:radial-gradient(90% 70% at 50% 0%,#1A1A1A 0,#0C0C0C 60%,#050505 100%);border-radius:14px;box-shadow:0 0 0 2px #2C2C2C,0 30px 60px rgba(0,0,0,.7)}
.tab{position:relative;width:1180px;height:820px;flex:none;overflow:hidden;box-sizing:border-box;display:grid;grid-template-rows:56px 1fr 92px;background:radial-gradient(90% 70% at 50% 0%,#1A1A1A 0,#0C0C0C 60%,#050505 100%);border-radius:28px;border:12px solid #050505;box-shadow:0 0 0 2px #2C2C2C,0 30px 60px rgba(0,0,0,.7)}
.top{display:flex;align-items:center;gap:18px;padding:0 18px;border-bottom:1px solid #1F1F1F;font-size:12px;color:#A3A3A3;white-space:nowrap}
.top .sep{width:1px;height:20px;background:#2A2A2A;flex:none}
.tabs{display:flex;gap:20px;margin-left:24px}
.tabs > span{font-size:11px;font-weight:700;letter-spacing:.1em;text-transform:uppercase;color:#6E6E6E;padding:17px 0;border-bottom:2px solid transparent}
.tabs > span.on{color:#F2F2F2;border-bottom-color:#F2F2F2}
.cols{display:grid;grid-template-columns:300px 1fr 360px;gap:14px;padding:14px 18px 0;min-height:0}
.col{display:flex;flex-direction:column;gap:14px;min-height:0;overflow:hidden}
.panel{background:linear-gradient(180deg,#181818,#111);border:1px solid #2C2C2C;border-radius:14px;box-shadow:inset 0 1px 0 rgba(255,255,255,.06),0 12px 30px rgba(0,0,0,.45)}
.ph{display:flex;justify-content:space-between;align-items:baseline;padding:12px 14px 8px}
.stage{flex:1;min-height:0;display:flex;flex-direction:column;gap:14px;overflow:hidden}
.hd{display:flex;align-items:baseline;gap:14px}
.foot{display:flex;align-items:center;gap:14px;padding:0 18px 14px}
.bn{flex:1;display:flex;align-items:center;gap:10px;height:48px;padding:0 16px;border-radius:12px;font-weight:700;font-size:15px;box-sizing:border-box}
.bn.you{background:#161616;border:1px solid #2C2C2C;color:#F2F2F2}
.bn.wait{background:#0A0A0A;border:1.5px dashed #5E5E5E;color:#D4D4D4}
.bn.warn{background:#0A0A0A;border:1.5px solid #FF9F1C;color:#F2F2F2}
.bn.hurt{background:#0A0A0A;border:1.5px solid #FF4D5E;color:#F2F2F2}
.bn.turn{background:#FFD60A;color:#0A0A0A;box-shadow:0 3px 0 #8A6F00}
.go{font:inherit;width:460px;flex:none;display:block;box-sizing:border-box;border:0;padding:6px;border-radius:12px;background:#EDEDED;color:#0A0A0A;text-align:left;cursor:pointer;box-shadow:0 4px 0 #8A8A8A,0 7px 0 #5A5A5A,0 14px 24px rgba(0,0,0,.5);transform-origin:50% 100%;transition:transform .1s,box-shadow .1s}
.go .in{display:flex;align-items:center;gap:12px;border:1.5px solid #0A0A0A;border-radius:8px;padding:6px 14px;height:42px;box-sizing:border-box}
.go .t{display:block;font-family:'Cinzel',Georgia,serif;font-weight:800;font-size:17px;line-height:1.05}
.go .s{display:block;font-size:11px;font-weight:600;color:#5A5A5A;margin-top:2px}
.go .chev{margin-left:auto;font-family:'Cinzel',serif;font-weight:800;font-size:22px}
.go:active{transform:perspective(500px) rotateX(14deg) translateY(4px);box-shadow:0 1px 0 #8A8A8A,0 2px 0 #5A5A5A}
.go.off{background:#141414;color:#8C8C8C;box-shadow:0 0 0 1.5px #2C2C2C;cursor:default;pointer-events:none}
.go.off .in{border:1.5px dashed #3A3A3A}
.go.off .s{color:#6E6E6E}
.go.red{background:#FF4D5E;box-shadow:0 4px 0 #8A2A33,0 7px 0 #5A1A20,0 14px 24px rgba(0,0,0,.5)}
.go.red .s{color:#3A0A10}
.btn{font:inherit;font-family:'Cinzel',Georgia,serif;font-size:12px;font-weight:800;color:#F2F2F2;background:#141414;border:0;border-radius:8px;height:34px;padding:0 12px;display:inline-flex;align-items:center;justify-content:center;gap:6px;cursor:pointer;white-space:nowrap;outline:1.5px solid #5A5A5A;outline-offset:-4px;box-shadow:0 0 0 1.5px #3A3A3A,0 3px 0 #000;flex:none}
.btn.lt{background:#EDEDED;color:#0A0A0A;outline-color:#0A0A0A;box-shadow:0 3px 0 #8A8A8A}
.sec{display:flex;flex-direction:column;gap:8px}
.row2{display:flex;gap:14px;align-items:flex-start}
.box{display:flex;align-items:center;gap:14px;padding:14px 16px;border-radius:12px;background:#0D0D0D;border:1px solid #333;font-size:14px}
.box > code{flex:1;font-family:'Chakra Petch',monospace;font-size:14px;color:#F2F2F2}
.edit{padding:14px 16px;border-radius:12px;background:#0D0D0D;border:1px solid #333;font-size:14px;line-height:1.55;color:#D4D4D4}
.edit > .lbl{display:block;margin-bottom:8px}
.rl{display:grid;grid-template-columns:130px 1fr;gap:12px;align-items:baseline;font-size:13px;line-height:1.45;padding:9px 12px;border-radius:10px;background:#121212;color:#D4D4D4}
.chk{display:flex;align-items:flex-start;gap:10px;font-size:13px;padding:10px 12px;border-radius:10px;background:#121212;line-height:1.45}
.chk > i{width:18px;height:18px;border-radius:5px;background:#EDEDED;color:#0A0A0A;display:grid;place-items:center;font-style:normal;font-weight:800;font-size:11px;flex:none;margin-top:1px}
.chk.warn{border:1.5px solid #FF9F1C}
.chk.warn > i{background:#FF9F1C}
.chk.pend > i{background:transparent;border:1.5px dashed #5E5E5E;color:#8C8C8C;box-sizing:border-box}
.co{display:flex;flex-direction:column;gap:8px;padding:0 14px 14px}
.note{padding:12px 14px;border-radius:12px;background:#EDEDED;color:#0A0A0A;font-size:13px;line-height:1.5;box-shadow:0 3px 0 #8A8A8A}
.note > .lbl{display:block;color:#5A5A5A;margin-bottom:4px}
.secret{display:flex;flex-direction:column;gap:6px;padding:12px 14px;border-radius:12px;background:#050505;border:1.5px dashed #5E5E5E;font-size:13px;line-height:1.5;color:#D4D4D4}
.secret > .lbl{color:#F2F2F2}
.hook{display:flex;flex-direction:column;gap:4px;padding:10px 12px;border-radius:10px;background:#141414;border:1px solid #2C2C2C;font-size:13px;line-height:1.45;color:#D4D4D4}
.hook > small{font-size:11px;color:#8C8C8C}
.hook.on{background:#EDEDED;color:#0A0A0A;border-color:#0A0A0A}
.hook.on > small{color:#5A5A5A}
.ans{display:flex;align-items:center;gap:10px;font-size:13px;padding:8px 12px;border-radius:10px;background:#121212;color:#D4D4D4}
.ans > b{width:80px;flex:none}
.ans > em{margin-left:auto;font-style:normal;font-size:11px;color:#8C8C8C;white-space:nowrap}
.st{font-size:10px;font-weight:700;letter-spacing:.08em;text-transform:uppercase;padding:4px 7px;border-radius:6px;white-space:nowrap;flex:none}
.st.ok{background:#EDEDED;color:#0A0A0A}
.st.todo{background:#FFD60A;color:#0A0A0A}
.st.warn{background:#0A0A0A;color:#FF9F1C;border:1.5px solid #FF9F1C}
.st.wait{border:1.5px dashed #5E5E5E;color:#A3A3A3}
.st.off{color:#5A5A5A;border:1px solid #2C2C2C}
.seat{display:flex;align-items:center;gap:12px;height:68px;padding:0 12px;margin:0 10px 6px;border-radius:12px;background:#121212;border:1px solid transparent}
.seat.cur{border-color:#F2F2F2;background:#1C1C1C}
.seat > .who{display:flex;flex-direction:column;gap:3px;flex:1;min-width:0}
.seat > .who > b{font-size:14px}
.seat > .who > small{font-size:11px;color:#8C8C8C;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.pf{display:inline-grid;place-items:end center;width:44px;height:56px;border-radius:9px;background:linear-gradient(180deg,#262626,#121212);border:2px solid #3A3A3A;box-shadow:0 3px 0 #000;box-sizing:border-box;flex:none;overflow:hidden}
.pf.none{background:transparent;border:2px dashed #3A3A3A;box-shadow:none}
.pf.down{border-color:#FF4D5E}
.pres{width:9px;height:9px;border-radius:50%;background:#3A3A3A;flex:none}
.pres.on{background:#F2F2F2;box-shadow:0 0 8px rgba(255,255,255,.6)}
.pstage{display:flex;align-items:flex-end;gap:14px;padding:14px;border-radius:14px;background:radial-gradient(60% 30% at 30% 92%,rgba(255,255,255,.08),transparent),#141414;border:1px solid #2C2C2C}
.abs{display:grid;grid-template-columns:repeat(6,1fr);gap:8px}
.ab{display:flex;flex-direction:column;align-items:center;gap:2px;padding:8px 0;border-radius:10px;background:#121212;border:1px solid #2C2C2C}
.ab > small{font-size:10px;font-weight:700;letter-spacing:.14em;color:#8C8C8C}
.ab > b{font-size:20px}
.ab > em{font-style:normal;font-size:11px;color:#A3A3A3}
.ab.fix{border:1.5px solid #F2F2F2;background:#1C1C1C}
.gems{display:flex;gap:14px;align-items:flex-end}
.gem{display:flex;flex-direction:column;align-items:center;gap:4px;font-size:10px;font-weight:600;letter-spacing:.12em;text-transform:uppercase;color:#8C8C8C}
.hand{display:flex;gap:12px}
.sent{display:flex;align-items:center;gap:10px;padding:12px 14px;border-radius:12px;background:#141414;border:1.5px dashed #5E5E5E;font-size:13px;color:#D4D4D4}
.spin{width:14px;height:14px;border-radius:50%;border:2px solid #3A3A3A;border-top-color:#F2F2F2;animation:spin 1s linear infinite;flex:none}
@keyframes spin{to{transform:rotate(360deg)}}
.toast{display:flex;flex-direction:column;gap:4px;padding:14px;border-radius:14px;background:#EDEDED;color:#0A0A0A;box-shadow:0 6px 0 #8A8A8A,0 20px 40px rgba(0,0,0,.6)}
.toast .lbl{color:#5A5A5A}
.art{height:178px;border-radius:12px;position:relative;background:radial-gradient(70% 80% at 50% 100%,rgba(10,10,10,.9),transparent),repeating-linear-gradient(135deg,#3A3A3A 0 2px,#2A2A2A 2px 9px);flex:none;display:flex;align-items:flex-end;padding:14px;box-sizing:border-box}
.tag{position:absolute;right:10px;top:10px;font-size:9px;font-weight:700;letter-spacing:.14em;background:#0A0A0A;padding:3px 6px;border-radius:4px}
.yt{display:flex;align-items:center;gap:10px;padding:6px 10px 6px 6px;border-radius:10px;background:#141414;border:1px solid #2C2C2C;font-size:12px}
.yt > .thumb{width:64px;height:36px;border-radius:5px;background:repeating-linear-gradient(135deg,#3A3A3A 0 2px,#262626 2px 7px);display:grid;place-items:center;flex:none;font-size:8px;font-weight:700;letter-spacing:.1em}
.yt > span:last-child{margin-left:auto;color:#8C8C8C}
.eq{display:flex;gap:2px;align-items:flex-end;height:14px}
.eq > i{width:3px;height:6px;background:#F2F2F2;animation:eq .8s ease-in-out infinite}
.eq > i:nth-child(2){animation-delay:-.3s}
.eq > i:nth-child(3){animation-delay:-.6s}
@keyframes eq{0%,100%{height:4px}50%{height:14px}}
.endc{position:absolute;inset:0;z-index:70;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:16px;text-align:center;background:rgba(5,5,5,.94);padding:30px}
.ph2{position:relative;width:390px;height:844px;border-radius:44px;overflow:hidden;border:10px solid #050505;box-shadow:0 0 0 2px #2C2C2C,0 30px 60px rgba(0,0,0,.7);background:radial-gradient(130% 70% at 50% 0%,#1A1A1A 0,#0C0C0C 60%,#050505 100%);box-sizing:border-box;display:flex;flex-direction:column;flex:none}
.pbn{margin:52px 12px 0;display:flex;align-items:center;gap:10px;padding:10px 14px;border-radius:12px;font-weight:700;font-size:14px}
.pbn.you{background:#EDEDED;color:#0A0A0A;box-shadow:0 3px 0 #8A8A8A}
.pbn.wait{background:#0A0A0A;border:1.5px dashed #5E5E5E;color:#D4D4D4}
.pbn.listen{background:#161616;border:1px solid #2C2C2C;color:#D4D4D4}
.pbn.warn{background:#0A0A0A;border:1.5px solid #FF9F1C;color:#F2F2F2}
.pbn.hurt{background:#0A0A0A;border:1.5px solid #FF4D5E;color:#F2F2F2}
.pbn.turn{background:#FFD60A;color:#0A0A0A;box-shadow:0 3px 0 #8A6F00}
.pbody{flex:1;display:flex;flex-direction:column;gap:12px;padding:16px;min-height:0;overflow:hidden}
.pft{display:flex;flex-direction:column;gap:8px;padding:10px 14px 30px}
.cb{font:inherit;display:block;box-sizing:border-box;width:100%;border:0;text-align:left;cursor:pointer;padding:6px;border-radius:12px;background:#EDEDED;color:#0A0A0A;box-shadow:0 4px 0 #8A8A8A,0 7px 0 #5A5A5A}
.cb .in{display:flex;align-items:center;gap:12px;border:1.5px solid #0A0A0A;border-radius:8px;padding:8px 12px;min-height:40px}
.cb .t{display:block;font-family:'Cinzel',serif;font-weight:800;font-size:17px}
.cb .s{display:block;font-size:11px;font-weight:600;color:#5A5A5A;margin-top:2px}
.cb .chev{margin-left:auto;font-family:'Cinzel',serif;font-weight:800;font-size:22px}
.cb.dk{background:#141414;color:#F2F2F2;box-shadow:0 0 0 1.5px #3A3A3A,0 4px 0 #000}
.cb.dk .in{border-color:#5A5A5A}
.cb.dk .s{color:#8C8C8C}
.cb.off{opacity:.45;pointer-events:none}
.pnav{display:flex;border-top:1px solid #1F1F1F;background:#060606;padding:2px 8px 18px}
.pnav > span{flex:1;font-size:12px;color:#8C8C8C;min-height:44px;display:grid;place-items:center;position:relative}
.pnav > span.on{color:#F2F2F2}
.pnav > span.on::before{content:'';position:absolute;top:0;left:50%;margin-left:-12px;width:24px;height:2px;background:#F2F2F2}
.dots{display:flex;gap:6px}
.dots > i{flex:1;height:4px;border-radius:2px;background:#2C2C2C}
.dots > i.on{background:#F2F2F2}
.chips{display:flex;flex-wrap:wrap;gap:8px}
.chip{font:inherit;display:inline-flex;align-items:center;gap:8px;padding:8px 12px;border-radius:10px;background:#141414;border:1.5px solid #2C2C2C;font-size:13px;color:#D4D4D4;cursor:pointer}
.chip.on{background:#EDEDED;border-color:#0A0A0A;color:#0A0A0A;box-shadow:0 3px 0 #8A8A8A}
.sw{font:inherit;width:40px;height:40px;border-radius:10px;border:2px solid #2C2C2C;cursor:pointer;padding:0;box-shadow:0 3px 0 #000}
.sw.on{border-color:#F2F2F2;box-shadow:0 0 0 2px #0A0A0A,0 0 0 4px #F2F2F2}
.tv{position:relative;width:960px;height:540px;flex:none;overflow:hidden;border-radius:10px;border:14px solid #050505;box-shadow:0 0 0 2px #2C2C2C,0 30px 60px rgba(0,0,0,.7);background:radial-gradient(90% 80% at 50% 0%,#1C1C1C 0,#0A0A0A 60%,#030303 100%);box-sizing:content-box}
.tvmini{position:relative;width:420px;height:236px;overflow:hidden;border-radius:8px;border:6px solid #050505;box-shadow:0 0 0 1.5px #2C2C2C;background:#0A0A0A;flex:none}
.tvmini > .inner{position:absolute;left:0;top:0;width:960px;height:540px;transform:scale(.4375);transform-origin:0 0}
.side{display:flex;flex-direction:column;gap:16px;width:420px;flex:none}
.think{padding:14px 16px;border-radius:14px;background:#EDEDED;color:#0A0A0A;font-size:14px;line-height:1.45;box-shadow:0 3px 0 #8A8A8A;min-height:92px;box-sizing:border-box;display:flex;align-items:center}
.why{padding:12px 14px;border-radius:12px;background:#141414;border:1px solid #2C2C2C;font-size:13px;line-height:1.5;color:#D4D4D4;min-height:110px;box-sizing:border-box}
.prog{display:flex;flex-wrap:wrap;gap:6px}
.prog > button{font:inherit;font-size:13px;font-weight:700;width:44px;height:36px;border-radius:8px;background:#141414;border:1px solid #2C2C2C;color:#6E6E6E;cursor:pointer}
.prog > button.done{color:#A3A3A3}
.prog > button.cur{background:#EDEDED;color:#0A0A0A;border-color:#0A0A0A}
.ctl{display:flex;gap:10px}
.ctl > button{font:inherit;font-size:13px;font-weight:600;height:40px;padding:0 16px;border-radius:10px;background:#141414;border:1px solid #3A3A3A;color:#D4D4D4;cursor:pointer}
@media (prefers-reduced-motion: reduce){.pm *{animation:none!important}}
'''

KEEP = {'on', 'off', 'done', 'cur', 'you', 'wait', 'warn', 'ok', 'todo', 'none', 'listen', 'hurt', 'turn', 'red', 'down'}


def imp(name, size, **kw):
    a = ' '.join(f'{k}="{v}"' for k, v in kw.items())
    return f'<dc-import name="{name}" {a} hint-size="{size[0]}px,{size[1]}px"></dc-import>'


def when(n, body):
    return '<sc-if value="{{is%d}}" hint-placeholder-val="{{ %s }}">%s</sc-if>' % (n, 'true' if n == 1 else 'false', body)


def iff(hole, body, default=False):
    return '<sc-if value="{{%s}}" hint-placeholder-val="{{ %s }}">%s</sc-if>' % (hole, 'true' if default else 'false', body)


def head(title, lbl='', size=26):
    return f'<div class="hd"><span class="ttl" style="font-size: {size}px">{title}</span><span class="lbl">{lbl}</span></div>'


def note(label, text):
    return f'<div class="note"><span class="lbl">{label}</span>{text}</div>'


def chk(text, kind=''):
    mark = {'warn': '!', 'pend': '·'}.get(kind, '✓')
    return f'<div class="chk {kind}"><i>{mark}</i><span>{text}</span></div>'


def cb(label, sub='', dark=False, act=None, cls=''):
    s = f'<span class="s">{sub}</span>' if sub else ''
    on = f' onClick="{{{{{act}}}}}"' if act else ''
    return f'<button type="button" class="cb {"dk" if dark else ""} {cls}"{on}><span class="in"><span style="flex: 1"><span class="t">{label}</span>{s}</span><span class="chev">›</span></span></button>'


def top(title, tabs, on, right='', brand='PROMPTUS'):
    t = ''.join(f'<span class="{"on" if x == on else ""}">{x}</span>' for x in tabs)
    return (f'<header class="top"><span class="ttl" style="font-size: 13px; letter-spacing: .3em; color: #F2F2F2">{brand}</span><span class="sep"></span>'
            f'<span class="ttl" style="font-size: 15px; color: #F2F2F2">{title}</span><span class="tabs">{t}</span><span style="flex: 1"></span>{right}</header>')


FOOT = '<div class="foot"><div class="bn {{bnCls}}"><span>{{bnTxt}}</span></div><button type="button" class="go {{mainCls}}" onClick="{{mainAct}}"><span class="in"><span style="flex: 1"><span class="t">{{mainLbl}}</span><span class="s">{{mainSub}}</span></span><span class="chev">›</span></span></button></div>'


def laptop(topbar, left, center, right, cols='300px 1fr 360px', cls='lap', overlay=''):
    return (f'<div class="{cls}">{topbar}<div class="cols" style="grid-template-columns: {cols}">'
            f'<div class="col">{left}</div><div class="col"><div class="stage">{center}</div></div><div class="col">{right}</div></div>{FOOT}{overlay}</div>')


def phone(blocks, nav=None, extra=''):
    """blocks: {n: (banner kind, banner text, body, foot)}; nav: {n: active tab} draws the tab bar."""
    out = ''
    for n, (k, b, body, ft) in blocks.items():
        navh = ''
        if nav and n in nav:
            navh = '<nav class="pnav">' + ''.join(f'<span class="{"on" if t == nav[n] else ""}">{t}</span>' for t in ('Jeu', 'Carte', 'Perso', 'Journal')) + '</nav>'
        b = b if b.startswith('<') else f'<span>{b}</span>'
        out += when(n, f'<div class="pbn {k}">{b}</div><div class="pbody">{body}</div><div class="pft">{ft}</div>{navh}')
    return '<div class="ph2">' + out + extra + '</div>'


def side(who, journey, n, tv=None):
    tvh = f'<span class="lbl">Sur la TV</span><div class="tvmini"><div class="inner">{tv}</div></div>' if tv else ''
    return f'''<div class="side">
<span class="lbl">Dans la tête de {who}</span><div class="think">{{{{thought}}}}</div>
{tvh}<span class="lbl">Pourquoi cet écran</span><div class="why">{{{{why}}}}</div>
<span class="lbl">{journey} · moment {{{{m}}}} sur {n}</span><div class="prog"><sc-for list="{{{{dots}}}}" as="d" hint-placeholder-count="{n}"><button type="button" class="{{{{d.cls}}}}" onClick="{{{{d.pick}}}}">{{{{d.n}}}}</button></sc-for></div>
<div class="ctl"><button type="button" onClick="{{{{restart}}}}">Recommencer</button></div>
</div>'''


def _js(x):
    if isinstance(x, str) and x.startswith('='):
        return x[1:]
    if isinstance(x, (list, tuple)):
        return '[' + ', '.join(_js(i) for i in x) + ']'
    return json.dumps(x, ensure_ascii=False)


def table(rows):
    """rows: {n: {key: value}}; a value starting with '=' is a JS expression evaluated in renderVals."""
    out = []
    for n, r in rows.items():
        out.append(f'{n}: {{ ' + ', '.join(f'{k}: {_js(v)}' for k, v in r.items()) + ' }')
    return '{\n      ' + ',\n      '.join(out) + '\n    }[m]'


def component(n, moments, fresh='{}', settled='{}', go='', pre='', vals='', full=(0, 0), seul=(0, 0), carry=()):
    """moments: {n: {bn: [kind, text], main: [label, sub, enabled, action JS], thought, why, ...}}.

    `fresh` and `settled` are JS object literals merged into the state (`settled` sees `m`);
    `go` runs after the state is reset for moment `m`; `pre` and `vals` add to renderVals.
    `carry` lists state keys a moment change keeps (choices the visitor made on the board).
    """
    return f'''
class Component extends DCLogic {{
  constructor(props) {{
    super(props);
    this.state = this.fresh();
    this.timers = [];
    const pm = Number(props && props.moment);
    if (pm >= 1 && pm <= {n}) Object.assign(this.state, this.settled(pm));
  }}
  fresh() {{ return Object.assign({{ m: 1, ended: false }}, {fresh}); }}
  // A frozen moment (storyboard) shows the screen once its people have acted.
  settled(m) {{ return Object.assign({{ m }}, {settled}); }}
  later(fn, ms) {{ this.timers.push(setTimeout(fn, ms)); }}
  clear() {{ this.timers.forEach((t) => clearTimeout(t)); this.timers = []; }}
  go(m) {{
    if (m > {n}) return;
    this.clear();
    const c = {{}};
    for (const k of {json.dumps(list(carry))}) c[k] = this.state[k];
    this.setState(Object.assign(this.fresh(), c, {{ m }}));
    {go}
  }}
  end() {{ this.clear(); this.setState({{ ended: true }}); }}
  renderVals() {{
    const s = this.state, m = s.m;
    {pre}
    const T = {table(moments)};
    const main = T.main;
    const v = {{
      m, ended: s.ended, notEnded: !s.ended,
      bnCls: T.bn[0], bnTxt: T.bn[1], thought: T.thought, why: T.why,
      mainLbl: main[0], mainSub: main[1], mainCls: (main[2] ? '' : 'off') + (main[4] ? ' ' + main[4] : ''), mainAct: () => {{ if (main[2] && main[3]) main[3](); }},
      dots: Array.from({{ length: {n} }}, (_, i) => ({{ n: i + 1, cls: i + 1 === m ? 'cur' : i + 1 < m ? 'done' : '', pick: () => this.go(i + 1) }})),
      restart: () => {{ this.clear(); this.setState(this.fresh()); }},
      {vals}
    }};
    for (const k in T) if (!(k in v) && k !== 'main' && k !== 'bn') v[k] = T[k];
    for (let i = 1; i <= {n}; i++) v['is' + i] = m === i;
    const seul = String(this.props.seul ?? false) === 'true';
    v.full = !seul;
    v.rootStyle = seul ? 'width: {seul[0]}px; height: {seul[1]}px; padding: 0' : 'width: {full[0]}px; height: {full[1]}px; padding: 60px 56px';
    return v;
  }}
}}
'''


def _place(name, layout):
    d = json.load(open('docs/design/canvas/canvas.json'))
    d['boards'].setdefault(name, layout)  # keep a layout set on the canvas
    if name not in d['order']:
        d['order'].append(name)
    json.dump(d, open('docs/design/canvas/canvas.json', 'w'), ensure_ascii=False, indent=2)


def board(name, title, prefix, parts, js, n, full, layout, css='', keep=()):
    html = f'''<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<title>{title}</title>
<script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
{FONTS}
<style>{CSS}{css}</style>
</helmet>
<div class="pm" style="{{{{rootStyle}}}}; position: relative; overflow: hidden; display: flex; gap: 40px; box-sizing: border-box; align-items: flex-start">
{parts}
</div>
</x-dc>
<script type="text/x-dc" data-dc-script data-props='{{"moment":{{"editor":"int","default":1,"min":1,"max":{n}}},"seul":{{"editor":"boolean","default":false}},"$preview":{{"width":{full[0]},"height":{full[1]}}}}}'>{js}</script>
</body>
</html>
'''
    out = f'docs/design/canvas/{name}.dc.html'
    o, _ren, dyn = scope(html, prefix, keep=KEEP | set(keep))
    open(out, 'w').write(o)
    _place(f'{name}.dc.html', layout)
    print(out, len(o), 'unsafe:', check(o, prefix, dyn))


SB_CSS = r'''
body{margin:0;background:#0A0A0A;overflow:hidden}
.pm{font-family:'Chakra Petch',system-ui,sans-serif;color:#F2F2F2;background:radial-gradient(120% 40% at 50% 0%,#1C1C1C 0,#0C0C0C 55%,#050505 100%);-webkit-font-smoothing:antialiased}
.ttl{font-family:'Cinzel',Georgia,serif;font-weight:800}
.lbl{font-size:11px;font-weight:600;letter-spacing:.2em;text-transform:uppercase;color:#8C8C8C}
.grid{display:grid;gap:56px 48px}
.mo{display:flex;flex-direction:column;gap:14px}
.mh{display:flex;align-items:baseline;gap:10px}
.mh > b{font-family:'Cinzel',serif;font-size:13px;width:28px;height:28px;border-radius:7px;display:inline-grid;place-items:center;background:#EDEDED;color:#0A0A0A;box-shadow:0 2px 0 #6E6E6E;flex:none}
.mh > span{font-family:'Cinzel',serif;font-weight:800;font-size:20px}
.fr{overflow:hidden;border-radius:10px;border:1px solid #2C2C2C;box-shadow:0 20px 40px rgba(0,0,0,.6)}
.cap{display:grid;grid-template-columns:110px 1fr;gap:6px 10px;font-size:13px;line-height:1.45;color:#D4D4D4;margin:0}
.cap > dt{font-size:10px;font-weight:700;letter-spacing:.16em;text-transform:uppercase;color:#8C8C8C;padding-top:3px}
.cap > dd{margin:0}
.rule{display:flex;gap:12px;align-items:flex-start;font-size:13px;color:#D4D4D4;line-height:1.45}
.rule > b{flex:none;width:24px;height:24px;border-radius:6px;background:#EDEDED;color:#0A0A0A;display:grid;place-items:center;font-family:'Cinzel',serif;box-shadow:0 2px 0 #6E6E6E}
'''


def storyboard(name, src, title, prefix, lbl, h1, intro, rules, frames, seul, box_w, cols, cap_keys, layout):
    """frames: [(title, caption 1, caption 2, caption 3)]; each frame is `src` frozen on its moment, scaled to box_w."""
    k = box_w / seul[0]
    box_h = round(seul[1] * k)
    W = 128 + cols * box_w + (cols - 1) * 48
    rows = -(-len(frames) // cols)
    cap_h = 150
    H = 230 + rows * (box_h + 42 + cap_h + 56)
    fr = ''.join(
        f'<div class="mo"><div class="mh"><b>{i}</b><span>{t}</span></div>'
        f'<div class="fr" style="width: {box_w}px; height: {box_h}px"><div style="width: {seul[0]}px; height: {seul[1]}px; transform: scale({k:.4f}); transform-origin: 0 0"><dc-import name="{src}" moment="{i}" seul="true" hint-size="{seul[0]}px,{seul[1]}px"></dc-import></div></div>'
        '<dl class="cap">' + ''.join(f'<dt>{ck}</dt><dd>{c}</dd>' for ck, c in zip(cap_keys, caps)) + '</dl></div>'
        for i, (t, *caps) in enumerate(frames, 1))
    rl = ''.join(f'<div class="rule"><b>{n}</b><span><strong style="color: #F2F2F2">{a}</strong> {b}</span></div>' for n, (a, b) in enumerate(rules, 1))
    html = f'''<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<title>{title}</title>
<script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
{FONTS}
<style>{SB_CSS}</style>
</helmet>
<div class="pm" style="width: {W}px; height: {H}px; position: relative; overflow: hidden">
<div style="padding: 48px 64px 32px; display: flex; gap: 80px; align-items: flex-end">
<div style="display: flex; flex-direction: column; gap: 10px; max-width: 1000px">
<div class="lbl">{lbl}</div>
<h1 class="ttl" style="margin: 0; font-size: 50px">{h1}</h1>
<p style="margin: 0; font-size: 16px; color: #A3A3A3; line-height: 1.5">{intro}</p>
</div>
<div style="display: flex; flex-direction: column; gap: 10px; max-width: 900px">{rl}</div>
</div>
<div class="grid" style="padding: 0 64px; grid-template-columns: repeat({cols}, {box_w}px)">{fr}</div>
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
    out = f'docs/design/canvas/{name}.dc.html'
    o, _ren, dyn = scope(html, prefix, keep=set())
    open(out, 'w').write(o)
    layout = dict(layout, w=W, h=H)
    _place(f'{name}.dc.html', layout)
    print(out, len(o), (W, H), 'unsafe:', check(o, prefix, dyn))
    return W, H
