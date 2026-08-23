import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'

import AppDialog from './AppDialog.vue'

describe('AppDialog', () => {
  it('渲染标题与默认 slot 内容', () => {
    const wrapper = mount(AppDialog, {
      props: { title: '添加评论 · #42' },
      slots: { default: '<p class="body">内容</p>' },
    })

    expect(wrapper.find('.dialog-title').text()).toBe('添加评论 · #42')
    expect(wrapper.find('.body').text()).toBe('内容')
  })

  it('提供 footer slot 时渲染操作区', () => {
    const wrapper = mount(AppDialog, {
      props: { title: 't' },
      slots: { footer: '<button class="ghost-btn">取消</button>' },
    })

    expect(wrapper.find('.dialog-actions').exists()).toBe(true)
    expect(wrapper.find('.ghost-btn').text()).toBe('取消')
  })

  it('点击遮罩空白处触发 close', async () => {
    const wrapper = mount(AppDialog, { props: { title: 't' } })

    await wrapper.find('.dialog-overlay').trigger('click')

    expect(wrapper.emitted('close')).toHaveLength(1)
  })
})
