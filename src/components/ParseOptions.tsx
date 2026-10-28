import { ChevronDown, SlidersHorizontal } from 'lucide-react'
import { useState } from 'react'
import { useApp } from '../context/AppContext'
import type { FeatureConfig, ParseOptions } from '../types'
import { Toggle } from './ui'

const featureLabels: Record<keyof FeatureConfig, [string, string]> = {
  enableTextCrossPage: ['合并跨页文字', 'Merge text across pages'],
  enableTableCrossPage: ['合并跨页表格', 'Merge tables across pages'],
  enableTitleLevelRecognition: ['识别标题层级', 'Recognize heading levels'],
  enableInlineImage: ['还原文中图片', 'Restore inline images'],
  enableTableImage: ['还原表中图片', 'Restore table images'],
  enableImageUnderstanding: ['启用图片理解', 'Enable image understanding'],
  enableImageOcr: ['识别图内文字', 'Recognize text in images'],
  keepHeaderFooter: ['保留页眉页脚', 'Keep headers and footers'],
}

export default function ParseOptionsPanel({
  options,
  onChange,
  formats,
  onFormatsChange,
}: {
  options: ParseOptions
  onChange: (options: ParseOptions) => void
  formats: string[]
  onFormatsChange: (formats: string[]) => void
}) {
  const { t, settings } = useApp()
  const [advanced, setAdvanced] = useState(false)
  const hasZip = formats.includes('zip')

  const toggleFormat = (format: string) => {
    const next = formats.includes(format) ? formats.filter((item) => item !== format) : [...formats, format]
    onFormatsChange(next)
    if (format === 'zip') {
      onChange({
        ...options,
        elementFormats: { ...options.elementFormats, image: next.includes('zip') ? 'file' : 'url' },
      })
    }
  }

  return (
    <div className="parse-options">
      <div className="field">
        <label>{t('new.formats')}</label>
        <div className="check-group">
          {[['markdown', 'Markdown'], ['json', 'JSON'], ['zip', 'ZIP + images']].map(([value, label]) => (
            <label key={value} className="check-chip">
              <input type="checkbox" checked={formats.includes(value)} onChange={() => toggleFormat(value)} />
              <span>{label}</span>
            </label>
          ))}
        </div>
      </div>
      <button type="button" className="advanced-toggle" onClick={() => setAdvanced(!advanced)}>
        <SlidersHorizontal size={16} />
        <span>{t(advanced ? 'new.simple' : 'new.advanced')}</span>
        <ChevronDown size={15} className={advanced ? 'rotated' : ''} />
      </button>
      {advanced && (
        <div className="advanced-panel">
          <div className="form-grid three-columns">
            <div className="field">
              <label htmlFor="image-format">Image</label>
              <select
                id="image-format"
                value={options.elementFormats.image}
                disabled={hasZip}
                onChange={(event) => onChange({
                  ...options,
                  elementFormats: { ...options.elementFormats, image: event.target.value as typeof options.elementFormats.image },
                })}
              >
                {hasZip && <option value="file">file (ZIP)</option>}
                {!hasZip && ['url', 'base64', 'none'].map((value) => <option key={value}>{value}</option>)}
              </select>
            </div>
            <div className="field">
              <label htmlFor="formula-format">Formula</label>
              <select
                id="formula-format"
                value={options.elementFormats.formula}
                onChange={(event) => onChange({
                  ...options,
                  elementFormats: { ...options.elementFormats, formula: event.target.value as typeof options.elementFormats.formula },
                })}
              >
                {['latex', 'mathml', 'ascii'].map((value) => <option key={value}>{value}</option>)}
              </select>
            </div>
            <div className="field">
              <label htmlFor="table-format">Table</label>
              <select
                id="table-format"
                value={options.elementFormats.table}
                onChange={(event) => onChange({
                  ...options,
                  elementFormats: { ...options.elementFormats, table: event.target.value as typeof options.elementFormats.table },
                })}
              >
                {['markdown', 'html', 'image'].map((value) => <option key={value}>{value}</option>)}
              </select>
            </div>
          </div>
          <div className="feature-grid">
            {(Object.keys(featureLabels) as (keyof FeatureConfig)[]).map((key) => (
              <Toggle
                key={key}
                checked={options.featureConfig[key]}
                label={featureLabels[key][settings?.language === 'en-US' ? 1 : 0]}
                onChange={(checked) => onChange({
                  ...options,
                  featureConfig: { ...options.featureConfig, [key]: checked },
                })}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  )
}
