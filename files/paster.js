const pasteInput = document.getElementById('input')
const pasteSubmit = document.getElementById('paste')
const filetypeInput = document.getElementById('filetype')

let lock = false

pasteInput.addEventListener('keyup', ev => {
  if (ev.key === 'Enter') {
    make_paste(pasteInput.value)
  }
})

pasteInput.addEventListener('paste', ev => {
  const paste = (ev.clipboardData || window.clipboardData).getData('text')
  make_paste(paste)
})

pasteSubmit.addEventListener('click', ev => {
  make_paste(pasteInput.value)
})

const make_paste = async (link) => {
  if (lock) return
  lock = true
  if (link.length > 0) {
    pasteInput.value = ''
    pasteInput.placeholder = 'Generating link...'
    const response = await fetch('/p', {
      method: 'POST',
      headers: {
        'Content-Type': 'text/plain; charset=utf-8'
      },
      body:
        link,
      })
    if (response.ok) {
      pasteInput.classList.remove('error')
      let linkData = await response.text()
        if (filetypeInput.value == "") {
            const linkData = linkData
        } else {
            linkData = linkData+ "." + filetypeInput.value
        }
      pasteInput.value = linkData
      pasteInput.placeholder = 'Data to paste...'
      pasteInput.select()
    } else {
      pasteInput.classList.add('error')
      pasteInput.value = ''
      pasteInput.placeholder = await response.text()
    }
  } else {
    pasteInput.classList.add('error')
    pasteInput.value = ''
    pasteInput.placeholder = 'Cannot make an empty paste.'
  }
  lock = false
}
