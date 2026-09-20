import os
from PIL import Image, ImageDraw, ImageFilter, ImageFont
H=os.path.expanduser
AR=H('~/Library/Fonts/IBMPlexSansArabic-Bold.ttf'); LA=H('~/Library/Fonts/IBMPlexSansArabic-Bold.ttf')
N=1024; S=3
def rgb(h): return tuple(int(h[i:i+2],16) for i in (1,3,5))
def canvas(top,bottom):
    im=Image.new('RGB',(N*S,N*S)); d=ImageDraw.Draw(im); t,b=rgb(top),rgb(bottom)
    for y in range(N*S):
        k=y/(N*S-1); d.line([(0,y),(N*S,y)],fill=tuple(round(a+(c-a)*k) for a,c in zip(t,b)))
    return im,d
def glyph(d,ch,font,h,cx,cy,fill):
    p=ImageFont.truetype(font,1000); l,t,r,b=p.getbbox(ch); f=ImageFont.truetype(font,round(1000*h*S/(b-t))); l,t,r,b=f.getbbox(ch)
    d.text((cx*S-(r-l)/2-l, cy*S-(b-t)/2-t),ch,font=f,fill=rgb(fill) if isinstance(fill,str) else fill)
def rr(d,x,y,w,h,r,fill): d.rounded_rectangle([x*S,y*S,(x+w)*S,(y+h)*S],radius=r*S,fill=rgb(fill))
def finish(im):
    flat=im.resize((N,N),Image.LANCZOS); B,M,C=824,100,186
    m=Image.new('L',(N*S,N*S),0); ImageDraw.Draw(m).rounded_rectangle([M*S,M*S,(M+B)*S,(M+B)*S],radius=C*S,fill=255); m=m.resize((N,N),Image.LANCZOS)
    out=Image.new('RGBA',(N,N),(0,0,0,0)); sh=Image.new('RGBA',(N,N),(0,0,0,0)); sh.paste((0,0,0,90),(0,12),m)
    out=Image.alpha_composite(out,sh.filter(ImageFilter.GaussianBlur(14)))
    p=Image.new('RGBA',(N,N),(0,0,0,0)); p.paste(flat.resize((B,B),Image.LANCZOS),(M,M)); p.putalpha(m)
    return Image.alpha_composite(out,p)

INK='#22303F'
# 1 — the split key: one keycap cut along its diagonal, the halves slid apart; Arabic on one, Latin on the other
def c1():
    im,d=canvas('#4A6485','#33465C'); K,R=560,120; x=y=(N-K)//2
    full=Image.new('L',(N*S,N*S),0); ImageDraw.Draw(full).rounded_rectangle([x*S,y*S,(x+K)*S,(y+K)*S],radius=R*S,fill=255)
    def half(upper,shift,face,edge):
        tri=Image.new('L',(N*S,N*S),0); g=13*S
        pts=[(0,0),(N*S,0),(0,N*S)] if upper else [(N*S,N*S),(N*S,0),(0,N*S)]
        ImageDraw.Draw(tri).polygon(pts,fill=255)
        # gap along the diagonal
        ImageDraw.Draw(tri).line([(N*S,0),(0,N*S)],fill=0,width=2*g)
        from PIL import ImageChops
        m=ImageChops.multiply(full,tri)
        for dy,col in ((22,edge),(0,face)):
            layer=Image.new('RGB',(N*S,N*S),rgb(col)); im.paste(layer,((shift)*S,(shift+dy)*S),ImageChops.offset(m,0,0))
    half(True,-26,'#FBFAF7','#CFC8BA'); half(False,26,'#E9D9B8','#C4B089')
    glyph(d,'A',LA,190,x+K*0.30-26,y+K*0.31-26,INK); glyph(d,'ع',AR,250,x+K*0.70+26,y+K*0.67+26,INK)
    return finish(im)
# 2 — the letter ب (first letter of بدّل) whose dot is a keycap
def c2():
    im,d=canvas('#4A6485','#33465C')
    glyph(d,'ٮ',AR,300,512,430,'#FBFAF7')
    k=150; rr(d,512-k/2,650+14,k,k,38,'#B99A5B'); rr(d,512-k/2,650,k,k,38,'#E9D9B8')
    return finish(im)
# 3 — one key, one badge: the Arabic key with the Latin letter riding on its corner
def c3():
    im,d=canvas('#4A6485','#33465C'); K,R=520,116; x,y=222,226
    rr(d,x,y+24,K,K,R,'#CFC8BA'); rr(d,x,y,K,K,R,'#FBFAF7'); glyph(d,'ع',AR,300,x+K/2-10,y+K/2-6,INK)
    r=150; cx,cy=x+K-40,y+K-30
    d.ellipse([(cx-r-18)*S,(cy-r-18)*S,(cx+r+18)*S,(cy+r+18)*S],fill=rgb('#3B5068'))
    d.ellipse([(cx-r)*S,(cy-r)*S,(cx+r)*S,(cy+r)*S],fill=rgb('#E9D9B8')); glyph(d,'A',LA,150,cx,cy,INK)
    return finish(im)
icons=[c1(),c2(),c3()]
for i,ic in enumerate(icons,1): ic.save(f'concept-{i}.png')
sizes=[300,128,64,32,16]; W=60+sum(s+40 for s in sizes); sheet=Image.new('RGB',(W,3*340),'#F6F4EF')
for r,ic in enumerate(icons):
    x=40
    for s in sizes:
        t=ic.resize((s,s),Image.LANCZOS); sheet.paste(t,(x,r*340+(340-s)//2),t); x+=s+40
sheet.save('sheet.png'); print('ok')
