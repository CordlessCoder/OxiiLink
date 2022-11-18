const pasteInput = document.getElementById('input')
const pasteSubmit = document.getElementById('paste')
const filetypeInput = document.getElementById('filetype')

function escapeRegExp(stringToGoIntoTheRegex) {
    return stringToGoIntoTheRegex.replace(/[-\/\\^$*+?.()|[\]{}]/g, '\\$&');
}

// const hrefRegex = new RegExp(escapeRegExp(window.location.href));
const hrefRegex = new RegExp("^"+escapeRegExp(window.location.href) + "\\/[A-z]{3}(\\.[A-z]+)?$");

let lock = false

pasteSubmit.addEventListener('click', ev => {
  make_paste(pasteInput.value)
})

const make_paste = async (paste) => {
  if (lock) return
  lock = true
  if (paste.length > 0) {
      if (hrefRegex.test(paste)) {
        pasteInput.classList.add('error')
      } else {

    pasteInput.value = ''
    pasteInput.placeholder = 'Generating paste...'
    const response = await fetch('/', {
      method: 'POST',
      headers: {
        'Content-Type': 'text/plain; charset=utf-8'
      },
      body:
        paste,
      })
    if (response.ok) {
      pasteInput.classList.remove('error')
      let pasteData = await response.text()
        if (filetypeInput.value == "") {
            pasteData = pasteData
        } else {
            pasteData = pasteData+ "." + filetypeInput.value
        }
      pasteInput.value = pasteData
      pasteInput.placeholder = 'Data to paste...'
      pasteInput.select()
    } else {
      pasteInput.classList.add('error')
      pasteInput.value = ''
      pasteInput.placeholder = await response.text()
    }
      }
  } else {
    pasteInput.classList.add('error')
    pasteInput.value = ''
    pasteInput.placeholder = 'Cannot make an empty paste.'
  }
  lock = false
}
