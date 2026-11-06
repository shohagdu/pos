import { useMemo } from 'react'

import { encodeBarcode } from '../../lib/barcode'

/** Quiet zone each side, in modules (scanners need white space around the bars). */
const QUIET = 10

/**
 * A barcode as an SVG that stretches to its box: bars scale with the label
 * width, so the same code prints on a 38 mm roll label or an A4 sheet.
 */
export function Barcode({ value, className = '', showText = true }: { value: string; className?: string; showText?: boolean }) {
  const encoded = useMemo(() => encodeBarcode(value), [value])
  if (!encoded) return <span className="font-mono text-[0.8em]">{value}</span>
  const { modules } = encoded
  let path = ''
  let i = 0
  while (i < modules.length) {
    if (modules[i] === '1') {
      let w = 1
      while (modules[i + w] === '1') w++
      path += `M${i + QUIET} 0h${w}v1h-${w}z`
      i += w
    } else i++
  }
  return (
    <div className={`flex min-h-0 flex-col items-stretch ${className}`}>
      <svg
        role="img"
        aria-label={value}
        viewBox={`0 0 ${modules.length + QUIET * 2} 1`}
        preserveAspectRatio="none"
        shapeRendering="crispEdges"
        className="min-h-0 w-full flex-1"
        data-symbology={encoded.symbology}
      >
        <path d={path} fill="#000" />
      </svg>
      {showText && <span className="text-center font-mono leading-none tracking-wider">{encoded.text}</span>}
    </div>
  )
}
