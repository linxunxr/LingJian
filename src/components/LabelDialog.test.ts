import { beforeEach, describe, expect, it, vi } from 'vitest'

const { actOnIssue } = vi.hoisted(() => ({ actOnIssue: vi.fn() }))
vi.mock('@/composables/useIssues', () => ({
  useIssues: () => ({ actOnIssue }),
  PRESET_LABELS: ['已修复', '无法复现', '高优先级', '待验证'],
}))

import { mount } from '@vue/test-utils'

import LabelDialog from './LabelDialog.vue'

beforeEach(() => {
  actOnIssue.mockReset()
  actOnIssue.mockResolvedValue(true)
})

function mountDialog(labels: readonly string[] = []) {
  return mount(LabelDialog, { props: { issueNumber: 42, labels } })
}

describe('LabelDialog', () => {
  it('打开时用当前标签回填草稿', () => {
    const wrapper = mountDialog(['高优先级'])

    expect(wrapper.find('.label-current').text()).toBe('高优先级')
    const checked = wrapper.findAll('.preset-label').find((l) => l.text() === '高优先级')!
    expect(checked.classes()).toContain('checked')
  })

  it('点预设标签切换选中态并更新全部标签展示', async () => {
    const wrapper = mountDialog(['高优先级'])

    const target = wrapper.findAll('.preset-label').find((l) => l.text() === '已修复')!
    // togglePreset 不读 DOM 勾选态、纯切换草稿，直接派发 change 驱动逻辑
    await target.find('input').trigger('change')

    expect(wrapper.find('.label-current').text()).toBe('高优先级、已修复')

    // 再触发一次取消
    const again = wrapper.findAll('.preset-label').find((l) => l.text() === '已修复')!
    await again.find('input').trigger('change')
    expect(wrapper.find('.label-current').text()).toBe('高优先级')
  })

  it('保存成功以整体替换提交并触发 saved/close', async () => {
    const wrapper = mountDialog(['高优先级'])

    const target = wrapper.findAll('.preset-label').find((l) => l.text() === '已修复')!
    await target.find('input').trigger('change')
    const save = wrapper.findAll('button').find((b) => b.text() === '保存')!
    await save.trigger('click')

    expect(actOnIssue).toHaveBeenCalledWith(42, 'setLabels', {
      labels: ['高优先级', '已修复'],
    })
    expect(wrapper.emitted('saved')).toEqual([[['高优先级', '已修复']]])
    expect(wrapper.emitted('close')).toHaveLength(1)
  })

  it('保存失败不关闭弹窗', async () => {
    actOnIssue.mockResolvedValue(false)
    const wrapper = mountDialog([])

    const save = wrapper.findAll('button').find((b) => b.text() === '保存')!
    await save.trigger('click')

    expect(wrapper.emitted('close')).toBeUndefined()
  })
})
