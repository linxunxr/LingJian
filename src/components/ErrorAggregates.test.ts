import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'

import type { ErrorAggregate } from '@/types'
import ErrorAggregates from './ErrorAggregates.vue'

const aggregates: ErrorAggregate[] = [
  {
    message: '灵气溢出',
    count: 12,
    firstSeen: '2026-08-23T10:00:00',
    lastSeen: '2026-08-23T11:30:00',
  },
  {
    message: '数据库写入失败',
    count: 3,
    firstSeen: '2026-08-23T09:00:00',
    lastSeen: '2026-08-23T09:05:00',
  },
]

describe('ErrorAggregates', () => {
  it('无聚合时展示空态文案', () => {
    const wrapper = mount(ErrorAggregates, { props: { aggregates: [] } })

    expect(wrapper.find('.empty').text()).toBe('暂无 ERROR/FATAL 日志')
    expect(wrapper.find('.count').exists()).toBe(false)
    expect(wrapper.findAll('.item')).toHaveLength(0)
  })

  it('渲染聚合列表：条目数、计数徽章与首末次时间', () => {
    const wrapper = mount(ErrorAggregates, { props: { aggregates } })

    expect(wrapper.find('.count').text()).toBe('2 类')
    const items = wrapper.findAll('.item')
    expect(items).toHaveLength(2)

    const first = items[0]
    expect(first.find('.message').text()).toBe('灵气溢出')
    expect(first.find('.count-badge').text()).toBe('12')
    expect(first.find('.meta').text()).toContain('首次 2026-08-23 10:00:00')
    expect(first.find('.meta').text()).toContain('末次 2026-08-23 11:30:00')
  })
})
