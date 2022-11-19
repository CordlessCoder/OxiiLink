const linkInput = document.getElementById('input')
const linkSubmit = document.getElementById('submit')

function escapeRegExp(stringToGoIntoTheRegex) {
    return stringToGoIntoTheRegex.replace(/[-\/\\^$*+?.()|[\]{}]/g, '\\$&');
}

const hrefRegex = new RegExp("^http(s)?:\/\/"+escapeRegExp(window.location.hostname.replace(/\/$/, "")) + "(:[\\d]+)?\\/(s\/)?[A-z\\d]{3}(\\.[A-z\\d]+)?$"); // if this regex matches, the URL is correct.

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
