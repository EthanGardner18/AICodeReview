# web_scraper.py
import requests
from bs4 import BeautifulSoup

def get_headings(url):
    response = requests.get(url)
    soup = BeautifulSoup(response.text, 'html.parser')
    headings = soup.find_all(['h1', 'h2', 'h3'])
    
    for heading in headings:
        print(heading.text.strip())

if __name__ == "__main__":
    url = "https://example.com"
    get_headings(url)

