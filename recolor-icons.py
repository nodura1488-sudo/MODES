from PIL import Image
import colorsys, os

src = Image.open('src-tauri/icons/icon-source.png').convert('RGBA')
out = Image.new('RGBA', src.size, (0, 0, 0, 0))
sp = src.load(); op = out.load()
w, h = src.size
for y in range(h):
    for x in range(w):
        r, g, b, a = sp[x, y]
        if a == 0:
            op[x, y] = (0, 0, 0, 0)
            continue
        rr, gg, bb = r / 255.0, g / 255.0, b / 255.0
        hh, ss, vv = colorsys.rgb_to_hsv(rr, gg, bb)
        if ss < 0.12:
            nr, ng, nb = rr, gg, bb
        elif vv < 0.25:
            nh = 0.995
            ns = min(ss, 0.55)
            nv = vv * 0.7
            nr, ng, nb = colorsys.hsv_to_rgb(nh, ns, nv)
        else:
            nh = 0.06 if hh < 0.62 else 0.995
            ns = min(1.0, max(0.68, ss))
            nv = min(1.0, vv * 1.1)
            nr, ng, nb = colorsys.hsv_to_rgb(nh, ns, nv)
        op[x, y] = (int(nr * 255), int(ng * 255), int(nb * 255), a)

out.save('src-tauri/icons/icon-source.png')
for size in (512, 256, 128, 64, 32):
    out.resize((size, size), Image.LANCZOS).save(f'src-tauri/icons/recolor-{size}.png')
out.resize((512, 512), Image.LANCZOS).save('src-tauri/icons/icon.png')
out.resize((256, 256), Image.LANCZOS).save('src-tauri/icons/128x128@2x.png')
out.resize((128, 128), Image.LANCZOS).save('src-tauri/icons/128x128.png')
out.resize((64, 64), Image.LANCZOS).save('src-tauri/icons/64x64.png')
out.resize((32, 32), Image.LANCZOS).save('src-tauri/icons/32x32.png')
out.save('src-tauri/icons/icon.ico', sizes=[(16,16),(24,24),(32,32),(48,48),(64,64),(128,128),(256,256)])
print('icons recolored')
