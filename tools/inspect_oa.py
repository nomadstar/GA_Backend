#!/usr/bin/env python3
"""
Inspecciona archivos en quickshift/src/datafiles y muestra hojas y primeras filas
Uso: python3 tools/inspect_oa.py [path/to/datafiles]
"""
import sys
import os
import zipfile
import xml.etree.ElementTree as ET

def get_datafiles_dir():
    # Prefer env
    if 'GA_DATAFILES_DIR' in os.environ:
        d = os.environ['GA_DATAFILES_DIR']
        if os.path.isdir(d):
            return d
    # CWD candidates
    cwd = os.getcwd()
    candidates = [
        os.path.join(cwd, 'quickshift', 'src', 'datafiles'),
        os.path.join(cwd, 'src', 'datafiles'),
        os.path.join(cwd, 'datafiles'),
    ]
    for c in candidates:
        if os.path.isdir(c):
            return c
    # fallback
    return os.path.join(cwd, 'quickshift', 'src', 'datafiles')


def read_shared_strings(z):
    try:
        data = z.read('xl/sharedStrings.xml')
    except KeyError:
        return []
    root = ET.fromstring(data)
    ns = {'main': 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
    strings = []
    for si in root.findall('.//main:si', ns):
        # collect text from t elements inside si (may have r/t runs)
        texts = []
        for t in si.findall('.//main:t', ns):
            texts.append(t.text or '')
        strings.append(''.join(texts))
    return strings


def get_sheet_names(z):
    try:
        data = z.read('xl/workbook.xml')
    except KeyError:
        return []
    root = ET.fromstring(data)
    ns = {'m': 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
    names = []
    for s in root.findall('.//m:sheets/m:sheet', ns):
        names.append(s.get('name'))
    return names


def sheet_files(z):
    # return list of (sheet_path, sheet_name)
    sheets = []
    names = get_sheet_names(z)
    # enumerate sheetN files present
    for name in z.namelist():
        if name.startswith('xl/worksheets/sheet') and name.endswith('.xml'):
            sheets.append(name)
    return sorted(sheets)


def extract_rows_from_sheet(z, sheet_path, shared):
    data = z.read(sheet_path)
    root = ET.fromstring(data)
    ns = {'m': 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
    rows = []
    for row in root.findall('.//m:sheetData/m:row', ns):
        cells = []
        for c in row.findall('m:c', ns):
            # check type
            t = c.get('t')
            v = c.find('m:v', ns)
            val = ''
            if v is not None and v.text is not None:
                if t == 's':
                    idx = int(v.text)
                    val = shared[idx] if idx < len(shared) else v.text
                else:
                    val = v.text
            else:
                # maybe inlineStr
                is_elem = c.find('m:is', ns)
                if is_elem is not None:
                    t_el = is_elem.find('.//m:t', ns)
                    if t_el is not None and t_el.text:
                        val = t_el.text
            cells.append(val)
        rows.append(cells)
    return rows


def inspect_file(path):
    print('\n== Inspecting:', path)
    try:
        with zipfile.ZipFile(path, 'r') as z:
            names = get_sheet_names(z)
            print('Sheet names:', names)
            shared = read_shared_strings(z)
            sfiles = sheet_files(z)
            print('Worksheet files:', sfiles[:10])
            for sf in sfiles:
                print('\n--', sf)
                rows = extract_rows_from_sheet(z, sf, shared)
                print('Rows count:', len(rows))
                for i, r in enumerate(rows[:10]):
                    print(i, r)
    except Exception as e:
        print('ERROR reading xlsx:', e)


def main():
    dirpath = sys.argv[1] if len(sys.argv) > 1 else get_datafiles_dir()
    print('Using datafiles dir:', dirpath)
    if not os.path.isdir(dirpath):
        print('Datafiles dir not found')
        return
    files = sorted(os.listdir(dirpath))
    print('Files found:', files)
    candidates = [f for f in files if 'oa' in f.lower() or 'oferta' in f.lower()]
    print('OA/Oferta candidates:', candidates)
    if not candidates:
        # show top xlsx files
        xlsx = [f for f in files if f.lower().endswith('.xlsx') or f.lower().endswith('.xls')]
        print('Other spreadsheet files:', xlsx)
        if xlsx:
            for f in xlsx[:3]:
                inspect_file(os.path.join(dirpath, f))
    else:
        for f in candidates:
            inspect_file(os.path.join(dirpath, f))

if __name__ == '__main__':
    main()
