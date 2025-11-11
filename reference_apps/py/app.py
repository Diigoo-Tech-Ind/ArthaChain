import os
from sdk.arthapy import ArthaPy

def main():
    base = os.environ.get('ARTHA_NODE', 'http://127.0.0.1:8080')
    sdk = ArthaPy(base)
    import sys
    if len(sys.argv) < 2:
        print('usage: upload <file> | info <artha://cid>')
        raise SystemExit(2)
    cmd = sys.argv[1]
    if cmd == 'upload':
        path = sys.argv[2]
        cid = sdk.upload_file(path)
        print('cid', cid)
    elif cmd == 'info':
        cid = sys.argv[2]
        print(sdk.info(cid))
    else:
        print('unknown command')
        raise SystemExit(2)

if __name__ == '__main__':
    main()


