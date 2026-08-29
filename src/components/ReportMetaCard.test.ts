import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'

import type { ReportMeta } from '@/types'
import ReportMetaCard from './ReportMetaCard.vue'

const fullMeta: ReportMeta = {
  title: '[用户反馈] 0.9.19 - 战斗界面卡死',
  userDescription: '战斗界面卡死，点技能没反应',
  appName: null,
  appVersion: '0.9.19',
  platform: 'windows',
  realm: '练气期',
  playTime: 3720,
  reportTime: '2026-08-08T18:15:04',
}

describe('ReportMetaCard', () => {
  it('meta 为 null 时整卡不渲染', () => {
    const wrapper = mount(ReportMetaCard, { props: { meta: null } })

    expect(wrapper.find('.report-meta').exists()).toBe(false)
  })

  it('展示标题与用户反馈文本', () => {
    const wrapper = mount(ReportMetaCard, { props: { meta: fullMeta } })

    expect(wrapper.find('.title').text()).toBe('[用户反馈] 0.9.19 - 战斗界面卡死')
    expect(wrapper.find('.description').text()).toBe('战斗界面卡死，点技能没反应')
  })

  it('环境条目按有值字段展示并格式化时长/时间', () => {
    const wrapper = mount(ReportMetaCard, { props: { meta: fullMeta } })

    const items = wrapper.findAll('.env-item').map((i) => i.text())
    expect(items).toEqual(['版本 0.9.19', '平台 windows', '境界 练气期', '时长 1小时2分', '上报时间 2026-08-08 18:15:04'])
  })

  it('本地导入场景（appName 有值、Issue 字段为空）只展示可用条目', () => {
    const meta: ReportMeta = {
      title: null,
      userDescription: null,
      appName: '星光音乐',
      appVersion: null,
      platform: 'android',
      realm: null,
      playTime: null,
      reportTime: '2026-08-08T18:15:04',
    }
    const wrapper = mount(ReportMetaCard, { props: { meta } })

    expect(wrapper.find('.description').exists()).toBe(false)
    const items = wrapper.findAll('.env-item').map((i) => i.text())
    expect(items).toEqual(['应用 星光音乐', '平台 android', '上报时间 2026-08-08 18:15:04'])
  })
})
