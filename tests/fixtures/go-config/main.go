package main

import "github.com/gin-gonic/gin"

func main() {
    router := gin.Default()
    router.GET("/login", login)
}

func login(ctx *gin.Context) {
    ctx.JSON(200, gin.H{"ok": true})
}
