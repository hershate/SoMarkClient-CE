import { ChevronRight } from 'lucide-react'
import { useState } from 'react'

export default function JsonTree({ value }: { value: unknown }) {
  const [limit, setLimit] = useState(50)
  if (Array.isArray(value)) {
    return (
      <div className="json-tree">
        {value.slice(0, limit).map((item, index) => <TreeNode key={index} name={String(index)} value={item} />)}
        {limit < value.length && <button type="button" className="button ghost" onClick={() => setLimit((current) => current + 50)}>Load more ({value.length - limit})</button>}
      </div>
    )
  }
  return <div className="json-tree"><TreeNode name="root" value={value} initialOpen /></div>
}

function TreeNode({ name, value, initialOpen = false }: { name: string; value: unknown; initialOpen?: boolean }) {
  const [open, setOpen] = useState(initialOpen)
  if (value === null || typeof value !== 'object') {
    return <div className="json-leaf"><span>{name}</span><code className={`json-${typeof value}`}>{JSON.stringify(value)}</code></div>
  }
  const entries = Array.isArray(value)
    ? value.map((item, index) => [String(index), item] as const)
    : Object.entries(value)
  return (
    <div className="json-node">
      <button type="button" onClick={() => setOpen(!open)}><ChevronRight size={13} className={open ? 'rotated' : ''} /><span>{name}</span><small>{Array.isArray(value) ? `[${entries.length}]` : `{${entries.length}}`}</small></button>
      {open && <div className="json-children">{entries.map(([key, child]) => <TreeNode key={key} name={key} value={child} />)}</div>}
    </div>
  )
}
