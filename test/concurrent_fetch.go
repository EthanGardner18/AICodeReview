// concurrent_fetch.go
package main

import (
    "fmt"
    "net/http"
    "sync"
)

func fetchURL(url string, wg *sync.WaitGroup) {
    defer wg.Done()
    resp, err := http.Get(url)
    if err != nil {
        fmt.Println("Error fetching URL:", err)
        return
    }
    fmt.Printf("Fetched %s with status code %d\n", url, resp.StatusCode)
    resp.Body.Close()
}

func main() {
    var wg sync.WaitGroup
    urls := []string{
        "https://example.com",
        "https://golang.org",
        "https://github.com",
    }

    for _, url := range urls {
        wg.Add(1)
        go fetchURL(url, &wg)
    }

    wg.Wait()
}

