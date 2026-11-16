from pathlib import Path
from PIL import Image, ImageDraw
root = Path(__file__).resolve().parent
source = Image.open('/Users/patricklarocque/.codex/generated_images/01a1e59c-a4be-7461-b743-6ba0e7d17d84/exec-7026ad8d-69e3-4ae5-97b3-5737de42ce73.png').convert('RGB')
source = source.resize((1440,1024),Image.Resampling.LANCZOS)
source.save(root/'source-normalized.jpg',quality=95)
for suffix in ['initial','final']:
    path = root / f'desktop-{suffix}.jpg'
    if not path.exists():
        continue
    actual = Image.open(path).convert('RGB')
    result = Image.new('RGB',(2880,1064),'#080d16')
    result.paste(source,(0,40))
    result.paste(actual,(1440,40))
    labels = ImageDraw.Draw(result)
    labels.text((20,12),'SELECTED REFERENCE — normalized to 1440 × 1024',fill='white')
    labels.text((1460,12),'BROWSER IMPLEMENTATION — 1440 × 1024',fill='white')
    result.save(root/f'comparison-{suffix}.jpg',quality=95)
    crop = Image.new('RGB',(2400,290),'#080d16')
    crop.paste(source.crop((240,510,1440,800)),(0,0))
    crop.paste(actual.crop((240,510,1440,800)),(1200,0))
    crop.save(root/f'detail-comparison-{suffix}.jpg',quality=95)
