const linkInput = document.getElementById('input')
const linkCopy = document.getElementById('input-copy-btn')
const linkSubmit = document.getElementById('submit')

let lock = false

linkInput.addEventListener('keyup', ev => {
  if (ev.key === 'Enter') {
    shortenLink(linkInput.value)
  }
})

linkInput.addEventListener('paste', ev => {
  const paste = (ev.clipboardData || window.clipboardData).getData('text')
  shortenLink(paste)
})

linkSubmit.addEventListener('click', ev => {
  shortenLink(linkInput.value)
})

const shortenLink = async (link) => {
  if (lock) return
  lock = true
  const isUrl = /(?:https?:\/\/).+\..+/
  if (isUrl.test(link)) {
    console.log(`Shortening ${link}`)
    linkInput.value = ''
    linkInput.placeholder = 'Generating link...'
    // eslint-disable-next-line no-undef
    const response = await fetch('/s', {
      method: 'POST',
      headers: {
        'Content-Type': 'text/plain; charset=utf-8'
      },
      body:
        link,
      })
    if (response.ok) {
      linkInput.classList.remove('error')
      const linkData = await response.text()
      linkInput.value = linkData
      linkInput.placeholder = 'Link to shorten...'
      linkInput.select()
    } else {
      linkInput.classList.add('error')
      linkInput.value = ''
      linkInput.placeholder = await response.text()
    }
  } else {
    linkInput.classList.add('error')
    linkInput.value = ''
    linkInput.placeholder = 'Are you absolutely sure that is a link?'
  }
  lock = false
}
