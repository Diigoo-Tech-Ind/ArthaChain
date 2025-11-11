import { ArthaJS } from '../../sdk/arthajs/index'

const base = process.env.ARTHA_NODE || 'http://127.0.0.1:8080'
const sdk = new ArthaJS(base)

async function main() {
  const [,, cmd, ...rest] = process.argv
  if (cmd === 'upload') {
    const file = rest[0]
    if (!file) throw new Error('usage: upload <file>')
    const cid = await sdk.uploadFile(file)
    console.log('cid', cid)
    return
  }
  if (cmd === 'info') {
    const cid = rest[0]
    if (!cid) throw new Error('usage: info <artha://cid>')
    const info = await sdk.info(cid)
    console.log(JSON.stringify(info, null, 2))
    return
  }
  console.error('usage: upload <file> | info <artha://cid>')
  process.exit(2)
}

main().catch((e) => { console.error(e); process.exit(1) })


