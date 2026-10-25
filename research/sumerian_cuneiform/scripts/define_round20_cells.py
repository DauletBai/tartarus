from pathlib import Path
import json,csv,datetime,hashlib
B=Path(__file__).resolve().parents[1]
def main():
 rows=[]
 def add(label,box,status,desc,groups=None,alt=''):
  rows.append({'region':label,'image':'sources/round19_image_C.jpg','box_xywh':box,'boundary_status':status,'description':desc,'candidate_groups':groups or [],'alternative':alt,'sound':None,'meaning':None})
 # These are photograph-space regions, not established ancient words.
 for n,box,status,desc,grp in [
 (1,[96,157,144,74],'partial','Fragmentary top region with diagonal grooves; no secure complete complex',[]),
 (2,[101,233,139,87],'clear','Pointed angular outline and internal stroke',['G01']),
 (3,[106,323,135,75],'clear','Pointed angular outline with less clear interior',['G01']),
 (4,[109,401,134,75],'clear','Similar pointed outline, lower surface damaged',['G01']),
 (5,[119,478,126,52],'uncertain','Short angled marks; cannot equate with complete G01',[]),
 (6,[135,531,110,57],'partial','Angled complex beside broken lower edge',[])]:add(f'L{n:02}',box,status,desc,grp)
 for n,box,status,desc,grp in [
 (1,[242,146,131,134],'partial','Horizontal line over large zigzag/angled strokes',[]),
 (2,[244,282,130,53],'clear','Arched horizontal stroke with downward segments',[]),
 (3,[246,338,128,91],'clear','Upright stroke with one short right branch',['G02']),
 (4,[248,434,127,85],'clear','Similar upright-and-branch complex',['G02']),
 (5,[253,523,126,69],'partial','Bent/angled strokes at lower edge',[])]:add(f'M{n:02}',box,status,desc,grp)
 for n,box,desc,grp in [(1,[377,123,170,169],'Round contour with crossbar',['G03']),(2,[378,296,170,106],'Round contour with angular interior',['G04']),(3,[381,405,168,103],'Round contour with crossbar and descending interior stroke',['G05']),(4,[387,512,163,88],'Small oval with several short internal strokes',['G06'])]:add(f'R{n:02}',box,'clear',desc,grp)
 for n,box,status,desc,alt in [
 (1,[550,99,77,79],'uncertain','Short grooves at upper edge','May merge with N02/N03 if cross grooves are not cell borders'),
 (2,[550,181,78,58],'uncertain','Short approximately parallel/angled strokes','May merge with N01/N03'),
 (3,[551,242,76,39],'uncertain','Small mark and cross groove','May merge with N01/N02'),
 (4,[551,284,78,88],'uncertain','Angular complex in narrow region','May merge with N05'),
 (5,[552,376,78,75],'uncertain','Angular complex bounded by cross groove','May merge with N04'),
 (6,[552,454,80,108],'clear','Tall framed shape with internal cross strokes','')]:add(f'N{n:02}',box,status,desc,[],alt)
 add('E01',[631,79,72,373],'partial','Narrow curved edge region with marks; internal segmentation not attempted',[], 'Not counted as a complete cell')
 (B/'data/round20_pilot_regions.json').write_text(json.dumps({'created_at':datetime.datetime.now().astimezone().isoformat(),'scope':'All22 mapped candidate regions in visible upper main view; not full object, all pixels or definite22 cells','reading_direction':'unestablished','regions':rows},ensure_ascii=False,indent=2)+'\n')
 with (B/'data/round20_pilot_regions.csv').open('w',encoding='utf-8-sig',newline='') as f:
  w=csv.DictWriter(f,fieldnames=['region','image','box_xywh','boundary_status','description','candidate_groups','alternative','sound','meaning']);w.writeheader();w.writerows(rows)
 print('22 mapped candidate regions; clear:',sum(r['boundary_status']=='clear' for r in rows))
if __name__=='__main__':main()
