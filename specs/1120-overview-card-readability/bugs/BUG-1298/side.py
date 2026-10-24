import sys
from PIL import Image, ImageDraw
S=sys.argv[1]
names=["p1-root","p2-ci","p3-regr","p4-idle-prog","p5-idle-prs","o1-action","o2-attn","o3-done","f1-expanded"]
for t in ["dark","light"]:
    pairs=[(Image.open(f"{S}/d/{t}-card{i}.png").convert("RGB"),Image.open(f"{S}/r/{t}-{n}.png").convert("RGB")) for i,n in enumerate(names)]
    for chunk,label in [(pairs[:5],"a"),(pairs[5:],"b")]:
        W=sum(max(d.width,a.width)+16 for d,a in chunk)+16
        H=max(d.height for d,a in chunk)+max(a.height for d,a in chunk)+48
        bg=(40,40,40) if t=="dark" else (200,200,200)
        img=Image.new("RGB",(W,H),bg); x=16
        top=max(d.height for d,a in chunk)+32
        for d,a in chunk:
            img.paste(d,(x,16)); img.paste(a,(x,top)); x+=max(d.width,a.width)+16
        img.save(f"{S}/side-{t}-{label}.png")
