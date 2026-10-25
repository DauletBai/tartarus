from pathlib import Path
import csv,json,datetime,collections
B=Path(__file__).resolve().parents[1];D=B/'data'
def main():
 p=[
 {'code':'P01','shape':'Прямой узкий рез/борозда','caution':'Форма следа; один видимый отрезок не обязательно равен одному движению инструмента.'},
 {'code':'P02','shape':'Криволинейный рез/дуга/контур','caution':'Замыкание и контакты уточняются по снимку; не назначено фонетическое значение.'},
 {'code':'P03','shape':'Угловатый/заострённый след','caution':'Может быть сочетанием прямых резов; не объявлен минимальным знаком.'},
 {'code':'P04','shape':'Глубокое заострённое углубление','caution':'Это категория формы; числовое значение не назначено.'},
 {'code':'P05','shape':'Округлое углубление','caution':'Не путать с повреждением; статус определяется по повторению и сохранности.'}]
 rows=[]
 def add(code,name,parts,evidence,image,assessment='provisional_graphic_type',uncertainty=''):
  rows.append({'code':code,'description':name,'visible_decomposition':parts,'evidence':evidence,'image_or_crop':image,'status':assessment,'uncertainty':uncertainty,'sound':None,'syllable':None,'word_meaning':None,'canonical_identity':None})
 add('G01','Заострённый угловатый контур с внутренними штрихами','Не менее двух сходящихся сторон; замыкание и внутренние штрихи проверяются отдельно','C01,C02,C03; грубые сходства A08,A10,A12,A14,A15','data/round19_C01_native.jpg',uncertainty='Семейство, не доказанный один знак; внутренние детали и замыкание варьируют.')
 add('G02a','Стоячий штрих с одним коротким ответвлением вправо','P01 + P01; сохранять ориентацию','C04; кандидат A12','data/round19_C04_native.jpg',uncertainty='Контакт с линиями разметки должен быть исключён.')
 add('G02b','Похожая конструкция с загнутым нижним участком','Как G02a + криволинейный нижний участок','C05','data/round19_C05_native.jpg',uncertainty='Нижний загиб может быть частью соседнего следа; не слит с G02a автоматически.')
 add('G03','Округлый контур с поперечным штрихом','P02 + P01','C06','data/round19_C06_native.jpg',uncertainty='Округлый контур сам по себе недостаточен для тождества.')
 add('G04','Округлый контур с угловатым внутренним рисунком','P02 + несколько угловатых внутренних участков','C07','data/round19_C07_native.jpg',uncertainty='Точное число движений инструмента по фотографии не установлено.')
 add('G05','Округлый контур, поперечина и нисходящий штрих','P02 + P01 + P01','C08','data/round19_C08_native.jpg',uncertainty='Контакт нисходящего штриха с контуром неясен; две топологические модели.')
 add('G06','Малый овал с короткими внутренними поперечинами','P02 + не менее двух ясных P01; возможный третий','C09','data/round19_C09_native.jpg',uncertainty='Количество и контакты штрихов не зафиксированы как точная числовая величина.')
 add('G07','Четырёхсторонняя рамка с внутренними перекладинами','Наружная угловатая рамка + внутренние P01','Два фрагмента A11','data/round20_G07a_native.jpg',uncertainty='Не объединяется с овалом G06 только из-за поперечин.')
 add('G08','Стоячая конструкция с предполагаемыми несколькими ответвлениями','Разложение не определено','A07, фрагмент G08','data/round20_G08_native.jpg','segmentation_unresolved','После увеличения нельзя уверенно отделить ответвления от рамки/нижнего участка; не засчитывается как подтверждённый отдельный тип.')
 add('G09','Пучок коротких параллельных резов без замкнутой рамки','Повтор P01; видимое количество в примере около пяти-шести','A11, фрагмент G09','data/round20_G09_native.jpg',uncertainty='Верхний/нижний рез может быть границей поля; количественное значение не назначено.')
 add('G10','Отдельное глубокое заострённое углубление','P04; внутренний след не установлен','A11 и A14; фрагмент G10 из A14','data/round20_G10_native.jpg',uncertainty='Роль неизвестна: не объявлен ни буквой, ни единицей счёта.')
 add('G11','Округлый контур со стоячей внутренней чертой','P02 + близкий к вертикальному P01','Два соседних примера A06','data/round20_G11_native.jpg',uncertainty='Не считается строгим совпадением G03 при разрешённом задним числом повороте.')
 add('G12','Заострённая рамка с плотным внутренним расчленением','P03/сходящиеся стороны + несколько пересечений P01','A12, фрагмент G12','data/round20_G12_native.jpg',uncertainty='Внешнее сходство с G01 не доказывает одинакового звука или функции.')
 out={'created_at':datetime.datetime.now().astimezone().isoformat(),'author_priority':'Inventory before words/syllables; no phonetic values assigned','geometric_trace_categories':p,'inventory':rows,'counts':{'working_graphic_types':12,'unresolved_candidate':1,'phonetic_values':0},'scope':'Not all ancient signs; local inventory from selected photographs. Original6-family heldout test retained separately; G02 refinement is subsequent and not used to rewrite frozen criteria.'}
 (D/'round20_graphic_inventory.json').write_text(json.dumps(out,ensure_ascii=False,indent=2)+'\n')
 with (D/'round20_graphic_inventory.csv').open('w',encoding='utf-8-sig',newline='') as f:
  w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
 # Abstract topology models; they describe alternative interpretations, not detected ancient strokes.
 models=[
 {'name':'upright_plus_branch','vertices':['top','junction','bottom','right'],'edges':[['top','junction'],['junction','bottom'],['junction','right']],'assumption':'All3 visible branches connect at one junction.'},
 {'name':'round_crossbar','vertices':['left','right'],'edges':[['left','right'],['left','right'],['left','right']],'assumption':'Two semicircular arcs and one crossbar meet at contour; idealised G03.'},
 {'name':'round_crossbar_dangling_branch','vertices':['left','right','mid','free'],'edges':[['left','right'],['left','right'],['left','mid'],['mid','right'],['mid','free']],'assumption':'G05 internal branch ends before reaching outer contour.'},
 {'name':'round_crossbar_joined_branch','vertices':['left','right','mid','bottom'],'edges':[['left','right'],['left','bottom'],['bottom','right'],['left','mid'],['mid','right'],['mid','bottom']],'assumption':'G05 branch reaches contour; alternative to previous model.'}]
 for m in models:
  deg=collections.Counter(v for edge in m['edges'] for v in edge);m['degree_multiset']=sorted(deg.values());m['connected_components_assumed']=1;m['cycle_rank']=len(m['edges'])-len(m['vertices'])+1
 (D/'round20_topology_models.json').write_text(json.dumps(models,ensure_ascii=False,indent=2)+'\n')
 print('Inventory:12 working graphic types +1 unresolved;5 shape categories;4 explicit topology models. No sounds assigned.')
if __name__=='__main__':main()
