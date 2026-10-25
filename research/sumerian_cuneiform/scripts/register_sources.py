#!/usr/bin/env python3
"""Record provenance and SHA-256 hashes of downloaded research sources."""
import csv
import hashlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCES = [
    ('P222607_card.html', 'https://cdli.earth/artifacts/222607', 'catalogue_and_editorial_transliteration', 'CDLI; see individual publications'),
    ('P222607_photo.jpg', 'https://cdli.earth/dl/photo/P222607_d.jpg', 'photographic_composite', 'CDLI credits Louvre; embedded Commons attribution; publication licence not yet verified'),
    ('P222607_lineart.jpg', 'https://cdli.earth/dl/lineart/P222607_ld.jpg', 'editorial_drawing', 'CDLI; Cone B = Ukg 4; bibliography checked 2026-10-23; not an independent photograph'),
    ('P222607_photo_reader.html', 'https://cdli.earth/artifacts/222607/reader/124168', 'image_reader_metadata', 'CDLI'),
    ('P222607_lineart_reader.html', 'https://cdli.earth/artifacts/222607/reader/42839', 'image_reader_metadata', 'CDLI'),
    ('inim_dictionary.html', 'https://oracc.museum.upenn.edu/epsd2/sux/o0031116', 'editorial_dictionary', 'ORACC/ePSD2; see project terms'),
    ('amargi_dictionary.html', 'https://oracc.museum.upenn.edu/epsd2/o0024072', 'editorial_dictionary', 'ORACC/ePSD2; see project terms'),
    ('Anunna_reference.html', 'https://oracc.museum.upenn.edu/amgg/listofdeities/anunna/index.html', 'secondary_research_reference', 'ORACC/AMGG; see project terms'),
]
OLD_NAMES = {row[0] for row in SOURCES}
SOURCES += [
    ('P222608_card.html', 'https://cdli.earth/artifacts/222608', 'catalogue_and_editorial_transliteration', 'CDLI; AO 3149; ex. 02'),
    ('P222608_photo.jpg', 'https://cdli.earth/dl/photo/P222608_d.jpg', 'catalogue_associated_photo', 'Physical attribution requires comparison with official museum views; do not infer identity solely from URL'),
    ('P222608_lineart.jpg', 'https://cdli.earth/dl/lineart/P222608_ld.jpg', 'editorial_drawing', 'Label Ukg 5 Cone C; see publications'),
    ('Louvre_AO3278.html', 'https://collections.louvre.fr/ark:/53355/cl010157057', 'museum_catalogue', 'Louvre; inventory and publication references'),
    ('Louvre_AO3149.html', 'https://collections.louvre.fr/ark:/53355/cl010120707', 'museum_catalogue', 'Louvre; title Cone C; five associated image URLs'),
    ('Louvre_photo_provenance_commons.html', 'https://commons.wikimedia.org/wiki/File:Clay_Cone_-_Louvre_Museum-_AO3278.jpg', 'image_provenance_discussion', 'User-written attribution discussion is not proof of restoration or substitution'),
    ('Louvre_AO3278_official.jpg', 'https://collections.louvre.fr/media/cache/large/0000000021/0000157057/0000222068_OG.JPG', 'official_museum_photo', 'Louvre; archive for examination; publication permission not established'),
    ('Louvre_AO3149_view1.jpg', 'https://collections.louvre.fr/media/cache/large/0000000021/0000120707/0000221694_OG.JPG', 'official_museum_photo', 'Louvre; archive for examination; publication permission not established'),
    ('Louvre_AO3149_view2.jpg', 'https://collections.louvre.fr/media/cache/large/0000000021/0000120707/0000221695_OG.JPG', 'official_museum_photo', 'Louvre; second view; publication permission not established'),
    ('Steinkeller_1991_download_response.html', 'https://www.ub.edu/ipoa/wp-content/uploads/2021/08/1991AuOrSteinkeller.pdf', 'failed_download_response', 'University temporary-unavailable HTML; NOT the paper PDF; bibliographic footnote checked separately via web index'),
    ('Kengir_dictionary.html', 'https://oracc.museum.upenn.edu/epsd2/sux/o0045501', 'editorial_dictionary', 'ORACC/ePSD2; written forms and bilingual equations'),
    ('emegir_dictionary.html', 'https://oracc.museum.upenn.edu/epsd2/o0026899', 'editorial_dictionary', 'ORACC/ePSD2; language designation; sheep sense must remain separate'),
    ('Sumer_name_Nimrud.html', 'https://oracc.museum.upenn.edu/nimrud/ancientkalhu/thewritings/sumerian/', 'secondary_research_reference', 'ORACC; historical chain of language designation'),
    ('P461950_Rimush_card.html', 'https://cdli.earth/artifacts/461950', 'composite_editorial_text', 'Not a photographed original; lines 9 and 33 contain editorial szu-me-ri2-am / szu-me-ri2-im'),
]

SOURCES += [
    ('Agade_dictionary.html', 'https://oracc.museum.upenn.edu/epsd2/names/cbd/qpn-x-places/x000000900.html', 'editorial_dictionary', 'ORACC/ePSD2; not a photograph'),
    ('Enki_Ninmah_composite.html', 'https://etcsl.orinst.ox.ac.uk/section1/c112.htm', 'composite_editorial_text', 'ETCSL Oxford; multiple witnesses, not one original'),
    ('Enki_Ninmah_translation.html', 'https://etcsl.orinst.ox.ac.uk/section1/tr112.htm', 'editorial_translation', 'ETCSL Oxford; not independently deciphered'),
    ('Igigi_reference.html', 'https://oracc.museum.upenn.edu/amgg/listofdeities/igigi/', 'secondary_research_reference', 'Nicole Brisch, AMGG; meanings change across texts'),
    ('RV_mandala01.html', 'https://gretil.sub.uni-goettingen.de/gretil/1_sanskr/1_veda/1_sam/1_rv/rv_hn01u.htm', 'electronic_text_edition', 'GRETIL; Aufrecht 1877 via Van Nooten/Holland and Eichler; not manuscript photo; source terms apply'),
    ('RV_mandala10.html', 'https://gretil.sub.uni-goettingen.de/gretil/1_sanskr/1_veda/1_sam/1_rv/rv_hn10u.htm', 'electronic_text_edition', 'GRETIL; ancient text distinguished from electronic edition; source terms apply'),
    ('Meluhha_AO22310.html', 'https://collections.louvre.fr/ark:/53355/cl010147038', 'museum_catalogue', 'Louvre; findspot attributed by style, not excavation context'),
    ('Meluhha_Possehl_2006.html', 'https://www.penn.museum/sites/expedition/shu-ilishus-cylinder-seal/', 'research_article', 'Gregory L. Possehl, Expedition 48(1), 2006; date differs from museum catalogue'),
]

SOURCES += [
    ('Shamhat_Interested_Readers.pdf', 'https://www.sbl-site.org/wp-content/uploads/2024/11/InterestedReaders_AitkenClinesMaier_SBL.pdf', 'research_collection_pdf', 'Daniel Bodi chapter; printed pp. 3-6 checked; PDF downloaded despite browser HTTP403; not original tablet photograph; publisher terms apply'),
    ('Hat_dictionary.html', 'https://sozdikqor.kz/soz?a=HAT&id=394281', 'editorial_dictionary', 'Kazakh word meaning; not evidence of the etymology of Shamhat'),
    ('Adapa_EA356_SEAL.html', 'https://seal.huji.ac.il/index.php/node/1557', 'catalogue_and_editorial_text', 'VAT 348 / EA356 / CDLI P270855; do not assign the whole composite opening to this witness'),
    ('Inana_Enki_translation.html', 'https://etcsl.orinst.ox.ac.uk/section1/tr131.htm', 'editorial_translation', 'ETCSL Oxford; segments E/F; includes destructive as well as constructive powers'),
    ('Enmerkar_Aratta_translation.html', 'https://etcsl.orinst.ox.ac.uk/section1/tr1823.htm', 'editorial_translation', 'ETCSL Oxford; lines134-155; speech unity should not be silently reversed into a narrative of fragmentation'),
    ('Enki_Ninhursag_translation.html', 'https://etcsl.orinst.ox.ac.uk/section1/tr111.htm', 'editorial_translation', 'ETCSL Oxford; ideal Dilmun is not a dated global historical period'),
    ('Nanshe_hymn_translation.html', 'https://etcsl.orinst.ox.ac.uk/section4/tr4141.htm', 'editorial_translation', 'ETCSL Oxford; social care within temple and royal setting'),
    ('Shuruppak_instructions_translation.html', 'https://etcsl.orinst.ox.ac.uk/section5/tr561.htm', 'editorial_translation', 'ETCSL Oxford; ethical sayings and acceptance of hierarchy coexist'),
]

NEW_2026_10_24 = [
    ('P000449_card.html', 'https://cdli.earth/artifacts/449', 'catalogue_and_editorial_transliteration', 'IM134358; W20421,2; Uruk IV; catalogue language Undetermined; not automatically phonetic Sumerian'),
    ('P000449_ORACC.html', 'https://oracc.museum.upenn.edu/dcclt/P000449', 'editorial_lexical_witness', 'Archaic Lu A; modern sign labels distinguished from ancient phonetic values'),
    ('P000449_lineart_reader.html', 'https://cdli.earth/artifacts/449/reader/416', 'image_reader_metadata', 'Links published drawing P000449_l.jpg; not a photograph'),
    ('P000449_lineart.jpg', 'https://cdli.earth/dl/lineart/P000449_l.jpg', 'editorial_drawing', 'Published drawing; examined for cell layout; reproduction permission not established'),
    ('P226284_card.html', 'https://cdli.earth/artifacts/226284', 'catalogue_and_editorial_transliteration', 'IM045428 and IM045429; tablet and canephore sections; preserve both object coordinates'),
    ('UrNamma34_Q000954.html', 'https://build-oracc.museum.upenn.edu/etcsri/Q000954', 'composite_and_grammar_model', '16 word tokens; editorial forms and morphological model stored separately; not one photographed original'),
    ('ETCSRI_introduction.html', 'https://oracc.museum.upenn.edu/etcsri/Introduction/index.html', 'research_project_methodology', 'Definition of corpus and distributional method; classification not treated as proof of ancestry'),
    ('ETCSRI_how_to.pdf', 'https://oracc.museum.upenn.edu/etcsri/downloads/how_to.pdf', 'research_methodology_pdf', 'Gabor Zolyomi; edited 2011 talk; printed pages3 and7-8 checked; reconstructed morphemes distinguished from written signs'),
    ('Woods_Visible_Language_download_response.html', 'https://isac.uchicago.edu/sites/default/files/uploads/shared/docs/oimp32.pdf', 'failed_download_response', 'HTML returned instead of PDF; only indexed extracts of Woods chapter seen; full volume not read'),
    ('Schmandt_Besserat_writing.html', 'https://sites.utexas.edu/dsb/tokens/the-evolution-of-writing/', 'author_research_publication', 'Token-to-writing reconstruction; token shapes do not identify spoken language or morphology; article claims require comparison with excavation reports'),
    ('WALS_fusion20.html', 'https://wals.info/chapter/20', 'comparative_typology_research', 'Bickel and Nichols; fusion exposition not a phylogenetic tree'),
]
NEW_2026_10_24 += [
    ('ArchaicLuA_Q000002.html', 'https://oracc.museum.upenn.edu/dcclt/Q000002', 'composite_editorial_lexical_text', 'Editorial composite; not a complete photographed tablet'),
    ('ArchaicLuA_CDLI_score.html', 'https://cdli.earth/artifacts/composites-score/Q000002', 'editorial_witness_alignment', 'P000449 linked here only at colophon; do not impute alignment of the full obverse'),
    ('P000106_card.html', 'https://cdli.earth/artifacts/106', 'catalogue_and_editorial_transliteration', 'Uruk III; first witness in score; two retained editorial records'),
    ('P000161_card.html', 'https://cdli.earth/artifacts/161', 'catalogue_and_editorial_transliteration', 'Uruk III; 31 editorial records; positive and negative matches retained'),
    ('P000163_card.html', 'https://cdli.earth/artifacts/163', 'catalogue_and_editorial_transliteration', 'Uruk III; nine editorial records; damage markers retained'),
    ('P251600_card.html', 'https://cdli.earth/artifacts/251600', 'catalogue_and_editorial_transliteration', 'Gudea102; MS2400; findspot uncertain; period is catalogue attribution not stratigraphic dating'),
    ('P251600_photo_reader.html', 'https://cdli.earth/artifacts/251600/reader/137398', 'image_reader_metadata', 'Links associated composite photograph; archaeological provenance remains uncertain'),
    ('P251600_lineart_reader.html', 'https://cdli.earth/artifacts/251600/reader/48147', 'image_reader_metadata', 'Links drawing; drawing not independent confirmation of reading'),
    ('P251600_photo.jpg', 'https://cdli.earth/dl/photo/P251600.jpg', 'associated_photographic_composite', 'MS2400 label visible; obverse/reverse and edge views; publication rights not verified'),
    ('P251600_lineart.jpg', 'https://cdli.earth/dl/lineart/P251600_l.jpg', 'editorial_drawing', 'Drawing separates eight-line text and modern reverse inscription; publication rights not verified'),
    ('P226284_photo.jpg', 'https://cdli.earth/dl/photo/P226284.jpg', 'associated_photographic_composite', 'Small image shows tablet and canephore; insufficient resolution for full sign collation; rights not verified'),
]
NEW_2026_10_24 += [
    ('ETCSRI_2026-10-24_json.zip', 'https://oracc.museum.upenn.edu/json/etcsri.zip', 'editorial_corpus_json_snapshot', 'Project metadata declares CC0; 1802 catalogue composites, not 1802 physical objects; image rights do not follow from dataset licence'),
]
NEW_2026_10_24 += [
    ('P226465_card.html', 'https://cdli.earth/artifacts/226465', 'catalogue_and_editorial_transliteration', 'VA10938 and VA10945; two object sections; no image in card; preservation not independently verified'),
    ('P226487_card.html', 'https://cdli.earth/artifacts/226487', 'catalogue_and_editorial_transliteration', 'VA15450; full editorial text conflicts with indexed publication excerpt describing fragment; unresolved'),
    ('P226725_card.html', 'https://cdli.earth/artifacts/226725', 'catalogue_and_editorial_transliteration', 'BM113896 and BM113866; two inventory objects in one card'),
    ('P226725_photo_reader.html', 'https://cdli.earth/artifacts/226725/reader/124564', 'image_reader_metadata', 'Associated image P226725_d.jpg; source association is not independent authentication'),
    ('P226725_photo.jpg', 'https://cdli.earth/dl/photo/P226725_d.jpg', 'associated_photographic_composite', 'Photo inspected; figurine113896 and stone tablet panels; publication rights not verified; no book use'),
    ('ETCSRI_Parsing.html', 'https://oracc.museum.upenn.edu/etcsri/Parsing/index.html', 'editorial_morphological_model', 'Reconstructed nominal and verbal templates distinguished from written signs'),
    ('Qazcorpus_learner.html', 'https://api.qazcorpus.kz/learner/', 'institutional_language_teaching_reference', 'Learner corpus; examples cite Zaisanbaev et al 2016; not an ancient written source'),
    ('Frayne_RIME_3_2_failed_download_response.html', 'https://enenuru.net/pdfs/RIME%203_2%20Frayne%2C%20Ur%20III%20Period%20%282112-2004%20BC%29%2C%20RIME%203-2%2C%201997.pdf', 'failed_download_response', 'Redirected to unrelated HTML; NOT Frayne PDF; indexed excerpt suggests a discrepancy requiring original verification'),
]
NEW_2026_10_24 += [
    ('SEAL_27667_CBS346.html', 'https://seal.huji.ac.il/index.php/node/27667', 'editorial_bilingual_witness', 'CBS346; literary letter; dated physical witness distinguished from narrated reign; damaged readings preserved'),
    ('P257792_card.html', 'https://cdli.earth/artifacts/257792', 'artifact_catalogue', 'CBS00346; Neo-Babylonian; findspot blank unlike SEAL Sippar; no CDLI transliteration displayed'),
    ('P257792_photo_reader.html', 'https://cdli.earth/artifacts/257792/reader/143505', 'image_reader_metadata', 'Links P257792.jpg; associated source not independent authentication'),
    ('P257792_lineart_reader.html', 'https://cdli.earth/artifacts/257792/reader/48528', 'image_reader_metadata', 'Links P257792_l.jpg; old editorial drawing not an independent witness'),
    ('P257792_photo.jpg', 'https://cdli.earth/dl/photo/P257792.jpg', 'associated_photographic_composite', 'Several views of one restored fragmentary tablet; number346 visible; rights for book publication not checked'),
    ('P257792_lineart.jpg', 'https://cdli.earth/dl/lineart/P257792_l.jpg', 'editorial_drawing', 'Old published drawing of CBS346; same drawing tradition as Langdon plate, not separate ancient evidence'),
    ('CBS346_Langdon1919_plate86.jpg', 'https://seal.huji.ac.il/sites/default/files/2021-03/pbs_10_4-138_page-0001.jpg', 'historical_publication_plate', 'Plate LXXXVI, text8; source scan inspected; age of drawing not date of artifact'),
    ('Viano_2016_unorthographic_writings.pdf', 'https://edizionicafoscari.unive.it/media/pdf/books/978-88-7543-417-5/978-88-7543-417-5-ch-06.pdf', 'author_research_chapter', 'Chapter4 pp141-228; introduction and selected passages only; pp153 and202 rendered and visually inspected; source tablets not inspected; research use only'),
]
NEW_2026_10_24 += [
    ('CTH315_concordance.html', 'https://hethport.net/hetkonk/hetkonk_abfrage.php?c=315', 'specialist_witness_concordance', 'Distinguishes RS25.421 from KUB4.2 and KUB4.97; reported import attribution distinguished from findspot'),
    ('UgariticaV_repository.html', 'https://repo.library.stonybrook.edu/xmlui/handle/11401/88656?show=full', 'institutional_repository_catalogue', 'University scan metadata; explicitly prohibits commercial use; research archive only'),
    ('UgariticaV_1968.pdf', 'https://repo.library.stonybrook.edu/xmlui/bitstream/handle/11401/88656/7779.pdf?isAllowed=y&sequence=1', 'original_research_publication_scan', 'Complete 80122241-byte scan,848 PDF pages; selected text310-314 and plates444-445 checked, not full volume read; repository noncommercial restriction'),
    ('P310404_ORACC.html', 'https://oracc.museum.upenn.edu/dcclt/P310404', 'editorial_bilingual_lexical_witness', 'YBC09868;327 edition rows including lacunae; no independent photograph obtained; preserve disagreements with CDLI'),
    ('P310404_card.html', 'https://cdli.earth/artifacts/310404', 'catalogue_and_editorial_transliteration', 'YBC09868; Old Babylonian, unknown findspot;286 extracted rows; not all ORACC positions present'),
    ('Ugarit_RSO_V20_bibliography.pdf', 'https://www.mission-ougarit.fr/wp-content/pdf/mission_ougarit_publication_rso_0520.pdf', 'excavation_mission_bibliography', 'Selected RS25.421 references checked; PDF page98 points to original text and autograph pages; not full volume read'),
]
NEW_2026_10_24 += [
    ('Viano_2016_Hattusa_chapter.pdf', 'https://edizionicafoscari.unive.it/media/pdf/books/978-88-7543-417-5/978-88-7543-417-5-ch-07.pdf', 'author_research_chapter', 'Chapter5; selected section5.3.2 pp256-262 read; p258 visually checked; Arnaud2007 not directly obtained; note1268 does not establish visible joins'),
    ('OSL_gu_homophones.html', 'https://oracc.museum.upenn.edu/osl/signlist/selpages/h001099.html', 'editorial_sign_catalogue', 'gu10 and mu listed under MU; catalogue sign identity is not contextual phonetic identity; OSL CC0'),
    ('OSL_MU_sign.html', 'https://oracc.museum.upenn.edu/osl/signlist/o0000457', 'editorial_sign_catalogue', 'MU sign page; source readings, not independent collation of the Ugarit tablet; OSL CC0'),
    ('OSL_am_homophones.html', 'https://oracc.museum.upenn.edu/osl/signlist/selpages/h000106.html', 'editorial_sign_catalogue', 'AM value lookup; not automatically a morpheme; OSL CC0'),
    ('OSL_ma_homophones.html', 'https://oracc.museum.upenn.edu/osl/signlist/selpages/h001929.html', 'editorial_sign_catalogue', 'MA value lookup; not automatically a morpheme; OSL CC0'),
    ('OSL_an_homophones.html', 'https://oracc.museum.upenn.edu/osl/signlist/selpages/h000133.html', 'editorial_sign_catalogue', 'AN value lookup; contextual reading must be checked; OSL CC0'),
    ('OSL_ku_homophones.html', 'https://oracc.museum.upenn.edu/osl/signlist/selpages/h001711.html', 'editorial_sign_catalogue', 'KU value lookup; contextual reading must be checked; OSL CC0'),
]
NEW_2026_10_24 += [
    ('Jagersma_Leiden_download_response.html', 'https://openaccess.leidenuniv.nl/bitstream/handle/1887/16107/Binnenwerkjagersma.pdf?sequence=2', 'failed_download_response', '843-byte Primo HTML returned instead of dissertation; not a PDF'),
    ('Jagersma_2010_mirror.pdf', 'https://sumerianastrology.com/wp-content/uploads/2024/08/Jagersma.pdf', 'author_dissertation_mirrored_copy', 'Leiden repository title sheet present;777 PDF pages; sections8.3.1-8.3.4 checked and p213 rendered; no hash comparison with current university copy; mirror site commentary not used'),
    ('P232300_card.html', 'https://cdli.earth/artifacts/232300', 'catalogue_and_editorial_transliteration', 'Gudea cylinderA;814 editorial records; selected by source-order protocol, not random sample'),
    ('P232275_card.html', 'https://cdli.earth/artifacts/232275', 'catalogue_and_editorial_transliteration', 'Gudea statueB; AO2;365 records; coordinate9:24 appears twice and both retained'),
    ('Louvre_AO2.html', 'https://collections.louvre.fr/ark:/53355/cl010119540', 'museum_catalogue', 'StatueB AO2; catalogue identity and excavation history checked; image publication rights not automatically granted'),
    ('P232300_photo_reader.html', 'https://cdli.earth/artifacts/232300/reader/128366', 'image_reader_metadata', 'Points to associated image P232300_d.jpg'),
    ('P232300_lineart_reader.html', 'https://cdli.earth/artifacts/232300/reader/45363', 'image_reader_metadata', 'Points to editorial drawing P232300_l.jpg'),
    ('P232275_photo1_reader.html', 'https://cdli.earth/artifacts/232275/reader/128351', 'image_reader_metadata', 'Points to associated photographic montage P232275.jpg'),
    ('P232275_photo2_reader.html', 'https://cdli.earth/artifacts/232275/reader/128352', 'image_reader_metadata', 'Points to frontal associated photograph P232275_d.jpg'),
    ('P232275_lineart_reader.html', 'https://cdli.earth/artifacts/232275/reader/45352', 'image_reader_metadata', 'Points to editorial drawing P232275_l.jpg'),
    ('P232300_photo.jpg', 'https://cdli.earth/dl/photo/P232300_d.jpg', 'associated_photographic_composite', 'Shows BOTH cylinderB left and cylinderA right plus thumbnail series; not full independent sign collation; rights not checked for book'),
    ('P232300_lineart.jpg', 'https://cdli.earth/dl/lineart/P232300_l.jpg', 'editorial_drawing', 'Numbered-column drawing; selected details inspected, not an independent photograph; book rights not checked'),
    ('P232275_photo1.jpg', 'https://cdli.earth/dl/photo/P232275.jpg', 'associated_photographic_montage', 'Contains contrasted/traced inscription panels and overall object; processing not independent sign evidence; book rights not checked'),
    ('P232275_photo2.jpg', 'https://cdli.earth/dl/photo/P232275_d.jpg', 'associated_photograph', 'Frontal view of headless statue and engraved robe; all target wedges not independently read; book rights not checked'),
    ('P232275_lineart.jpg', 'https://cdli.earth/dl/lineart/P232275_l.jpg', 'editorial_drawing', 'Nine-column inscription layout and temple plan; selected details inspected; book rights not checked'),
]
NEW_2026_10_24 += [
    ('P226422_card.html', 'https://cdli.earth/artifacts/226422', 'image_availability_catalogue', 'Round9 fixed cohort; no image; individual editorial text not exposed in this pass'),
    ('P226524_card.html', 'https://cdli.earth/artifacts/226524', 'image_availability_catalogue', 'Round9 fixed cohort; no image; individual editorial text not exposed in this pass'),
    ('P226526_card.html', 'https://cdli.earth/artifacts/226526', 'image_availability_catalogue', 'Round9 fixed cohort; no image; individual editorial text not exposed in this pass'),
    ('P226887_card.html', 'https://cdli.earth/artifacts/226887', 'image_availability_catalogue', 'Separate round9b frame; drawing only; not selected for photographic test'),
    ('P226912_card.html', 'https://cdli.earth/artifacts/226912', 'image_availability_catalogue', 'Separate round9b frame; no image; not selected'),
    ('P342808_card.html', 'https://cdli.earth/artifacts/342808', 'catalogue_and_editorial_transliteration', 'Selected by photo availability; individual rows exposed only after local visual freeze; uncertain anonymous identity; no independent archaeological context verified'),
    ('P499399_card.html', 'https://cdli.earth/artifacts/499399', 'image_availability_catalogue', 'Photo exists but image and individual editorial text not opened in round9b'),
    ('P226887_reader.html', 'https://cdli.earth/artifacts/226887/reader/44604', 'image_reader_metadata', 'Points to drawing P226887_l.jpg; drawing itself not opened'),
    ('P342808_photo_reader.html', 'https://cdli.earth/artifacts/342808/reader/168600', 'image_reader_metadata', 'Points to P342808.jpg; associated image not independent authentication'),
    ('P499399_reader.html', 'https://cdli.earth/artifacts/499399/reader/217954', 'image_reader_metadata', 'Points to P499399_d.jpg; picture not opened, retained as potential future candidate'),
    ('P342808_photo.jpg', 'https://cdli.earth/dl/photo/P342808.jpg', 'associated_photograph', '222x459 pixels; image-first observation frozen before individual rows; unresolved sign boundaries; copyright anonymous; no book publication permission established'),
]
SOURCES += NEW_2026_10_24
NEW_NAMES_24 = {row[0] for row in NEW_2026_10_24}

with (ROOT / 'SOURCE_REGISTER.csv').open('w', encoding='utf-8-sig', newline='') as out:
    writer = csv.writer(out)
    writer.writerow(['local_file', 'source_url', 'retrieved_date_Asia_Qostanay', 'type', 'bytes', 'sha256', 'rights_and_notes'])
    for name, url, kind, rights in SOURCES:
        path = ROOT / 'sources' / name
        data = path.read_bytes()
        if len(data) < 100:
            raise ValueError(f'Suspicious empty source: {name}')
        if name.endswith('.jpg') and not data.startswith(b'\xff\xd8'):
            raise ValueError(f'Expected JPEG signature: {name}')
        if name.endswith('.pdf') and not data.startswith(b'%PDF'):
            raise ValueError(f'Expected PDF signature: {name}')
        if name.endswith('.zip') and not data.startswith(b'PK'):
            raise ValueError(f'Expected ZIP signature: {name}')
        date = '2026-10-24' if name in NEW_NAMES_24 else ('2026-10-22' if name in OLD_NAMES else '2026-10-23')
        writer.writerow([f'sources/{name}', url, date, kind, len(data), hashlib.sha256(data).hexdigest(), rights])
print(f'Registered {len(SOURCES)} sources.')
